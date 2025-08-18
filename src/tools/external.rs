use crate::graph::DependencyGraph;
use crate::imports::ModuleOrigin;
use crate::pyproject;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

#[derive(Debug)]
pub struct ExternalAnalysisResult {
    pub frequency_analysis: Vec<DependencyUsage>,
    pub summary: ExternalDependencySummary,
    pub undeclared_dependencies: Vec<String>,
    pub unused_dependencies: Vec<String>,
    pub declared_externals_count: usize,
}

#[derive(Debug)]
pub struct DependencyUsage {
    pub package_name: String,
    pub usage_count: usize,
    pub used_by_modules: Vec<String>,
}

#[derive(Debug)]
pub struct ExternalDependencySummary {
    pub total_used_packages: usize,
}

pub fn analyze_external_dependencies(graph: &DependencyGraph) -> Result<ExternalAnalysisResult> {
    let used_externals = pyproject::get_used_externals()?;
    let frequency_analysis = collect_package_usage(graph, &used_externals)?;
    let declared_deps = pyproject::get_declared_dependencies()?;
    let (undeclared_dependencies, unused_dependencies) =
        analyze_dependency_gaps(&frequency_analysis, &declared_deps)?;

    let summary = ExternalDependencySummary {
        total_used_packages: frequency_analysis.len(),
    };

    Ok(ExternalAnalysisResult {
        frequency_analysis,
        summary,
        undeclared_dependencies,
        unused_dependencies,
        declared_externals_count: used_externals.len(),
    })
}

/// Collect usage statistics for external packages across internal modules
fn collect_package_usage(graph: &DependencyGraph, used_externals: &[String]) -> Result<Vec<DependencyUsage>> {
    let stdlib_modules = get_python_standard_library_modules();
    let mut package_usage: HashMap<String, Vec<String>> = HashMap::new();

    // Add manually declared external packages from .used-externals.txt
    for package_name in used_externals {
        // Skip Python standard library modules
        if !stdlib_modules.contains(package_name) {
            package_usage
                .entry(package_name.clone())
                .or_default()
                .push("(declared)".to_string());
        }
    }

    // Count usage of external packages across internal modules
    for module in graph.all_modules() {
        if module.origin == ModuleOrigin::Internal {
            let dependencies = graph.get_dependencies_with_types(module)?;

            for (dep_module, _dep_type) in dependencies {
                // Check if this dependency is external by looking for a module with External origin
                if let Some(external_module) = graph
                    .all_modules()
                    .find(|m| m.canonical_path == dep_module && m.origin == ModuleOrigin::External)
                {
                    // Extract root package name (e.g., numpy.testing -> numpy)
                    let package_name = extract_root_package_name(&external_module.canonical_path);

                    // Skip Python standard library modules
                    if stdlib_modules.contains(&package_name) {
                        continue;
                    }

                    package_usage
                        .entry(package_name)
                        .or_default()
                        .push(module.canonical_path.clone());
                }
            }
        }
    }

    // Convert to DependencyUsage and sort by usage count
    let mut frequency_analysis: Vec<DependencyUsage> = package_usage
        .into_iter()
        .map(|(package_name, mut used_by_modules)| {
            used_by_modules.sort();
            used_by_modules.dedup();

            DependencyUsage {
                package_name,
                usage_count: used_by_modules.len(),
                used_by_modules,
            }
        })
        .collect();

    // Sort by usage count (descending), then by name (ascending)
    frequency_analysis.sort_by(|a, b| {
        b.usage_count
            .cmp(&a.usage_count)
            .then_with(|| a.package_name.cmp(&b.package_name))
    });

    Ok(frequency_analysis)
}

/// Compare used packages against declared dependencies to find gaps
fn analyze_dependency_gaps(
    frequency_analysis: &[DependencyUsage],
    declared_deps: &[String],
) -> Result<(Vec<String>, Vec<String>)> {
    let declared_deps_set: HashSet<&str> = declared_deps.iter().map(String::as_str).collect();

    // Pre-fetch all package mappings once
    let mapping = build_complete_mapping(declared_deps)?;

    // Resolve import names to package names using pre-built mapping
    let resolved_used_deps: HashSet<String> = frequency_analysis
        .iter()
        .map(|dep| resolve_import_to_package_name(&mapping, &dep.package_name))
        .collect();

    // Find undeclared dependencies (used but not declared in pyproject.toml)
    let mut undeclared_dependencies: Vec<String> = resolved_used_deps
        .iter()
        .filter(|dep| !declared_deps_set.contains(dep.as_str()))
        .cloned()
        .collect();
    undeclared_dependencies.sort();

    // Find unused dependencies (declared but not used)
    let mut unused_dependencies: Vec<String> = declared_deps_set
        .iter()
        .filter(|dep| !resolved_used_deps.contains(**dep))
        .map(|s| s.to_string())
        .collect();
    unused_dependencies.sort();

    Ok((undeclared_dependencies, unused_dependencies))
}

/// Cached Python standard library modules
static PYTHON_STDLIB_MODULES: OnceLock<HashSet<String>> = OnceLock::new();

/// Get Python standard library modules by calling Python subprocess
fn get_python_standard_library_modules() -> &'static HashSet<String> {
    PYTHON_STDLIB_MODULES.get_or_init(|| {
        // Try python first (more common on Windows), then python3
        for python_cmd in ["python", "python3"] {
            match std::process::Command::new(python_cmd)
                .args([
                    "-c",
                    "import sys; print('\\n'.join(sys.stdlib_module_names))",
                ])
                .output()
            {
                Ok(output) if output.status.success() => {
                    let result: HashSet<String> = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .map(|line| line.trim().to_string())
                        .filter(|line| !line.is_empty())
                        .collect();

                    if !result.is_empty() {
                        return result;
                    }
                }
                _ => continue, // Try next command
            }
        }

        // If both fail, return empty set and warn user
        println!("Warning: Could not detect Python stdlib modules. Install Python or ensure it's in PATH.");
        HashSet::new()
    })
}

/// Extracts the root package name from a module path.
/// Examples: numpy.testing.utils -> numpy, scipy.stats -> scipy
fn extract_root_package_name(module_path: &str) -> String {
    module_path
        .split('.')
        .next()
        .unwrap_or(module_path)
        .to_string()
}

/// Package import mapping with static fallback and API results
#[derive(Debug, Clone)]
struct PackageImportMapping {
    /// Static mappings for common packages (import_name -> package_name)
    static_mappings: HashMap<String, String>,
    /// API results cache (import_name -> package_name)
    api_mappings: HashMap<String, String>,
}

impl PackageImportMapping {
    fn new() -> Result<Self> {
        Ok(Self {
            static_mappings: load_static_package_mappings()?,
            api_mappings: HashMap::new(),
        })
    }

    /// Resolve import name to package name using pre-built mapping
    fn resolve_import_to_package(&self, import_name: &str) -> String {
        let normalized_name = import_name.to_lowercase();
        
        // Check static mappings first (case-insensitive)
        if let Some(package_name) = self.static_mappings.get(&normalized_name) {
            return package_name.clone();
        }
        
        // Also check original case for exact matches
        if let Some(package_name) = self.static_mappings.get(import_name) {
            return package_name.clone();
        }

        // Check API results (case-insensitive)
        if let Some(package_name) = self.api_mappings.get(&normalized_name) {
            return package_name.clone();
        }
        
        // Also check original case for exact matches
        if let Some(package_name) = self.api_mappings.get(import_name) {
            return package_name.clone();
        }

        // Fall back to original import name if no mapping found
        import_name.to_string()
    }

    /// Add a mapping from import name to package name
    fn add_mapping(&mut self, import_name: String, package_name: String) {
        // Store both original case and lowercase for case-insensitive lookup
        let normalized_name = import_name.to_lowercase();
        self.api_mappings.insert(import_name.clone(), package_name.clone());
        if normalized_name != import_name {
            self.api_mappings.insert(normalized_name, package_name);
        }
    }
}

/// Pre-fetch API mappings for declared packages with progress bar
fn build_complete_mapping(declared_packages: &[String]) -> Result<PackageImportMapping> {
    let mut mapping = PackageImportMapping::new()?;

    if declared_packages.is_empty() {
        return Ok(mapping);
    }

    let pb = ProgressBar::new(declared_packages.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );
    pb.set_message("Fetching package mappings");

    for package_name in declared_packages {
        if let Ok(import_names) = query_pypi_for_imports(package_name) {
            for import_name in import_names {
                mapping.add_mapping(import_name, package_name.clone());
            }
        }

        pb.inc(1);
    }

    pb.finish_and_clear();
    Ok(mapping)
}

/// Main resolver function to convert import name to package name
fn resolve_import_to_package_name(mapping: &PackageImportMapping, import_name: &str) -> String {
    mapping.resolve_import_to_package(import_name)
}

/// JSON structure for package mappings file
#[derive(serde::Deserialize)]
struct PackageMappingsJson {
    import_to_package: HashMap<String, String>,
}

/// Loads static mapping table from JSON file
fn load_static_package_mappings() -> Result<HashMap<String, String>> {
    let json_content = include_str!("package_mappings.json");
    let mappings: PackageMappingsJson = serde_json::from_str(json_content)?;
    Ok(mappings.import_to_package)
}

/// PyPI API response structure
#[derive(serde::Deserialize)]
struct PyPIResponse {
    info: PyPIInfo,
}

#[derive(serde::Deserialize)]
struct PyPIInfo {
    #[serde(default)]
    top_level: Vec<String>,
}

/// Queries PyPI API to get top-level modules for a package with retry logic
fn query_pypi_for_imports(package_name: &str) -> Result<Vec<String>> {
    const MAX_RETRIES: u32 = 2;

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let url = format!("https://pypi.org/pypi/{}/json", package_name);

    for attempt in 0..=MAX_RETRIES {
        match client.get(&url).send() {
            Ok(response) if response.status().is_success() => {
                match response.json::<PyPIResponse>() {
                    Ok(pypi_response) => {
                        if !pypi_response.info.top_level.is_empty() {
                            return Ok(pypi_response.info.top_level);
                        } else {
                            // No top_level in response, use fallback
                            return Ok(vec![package_name.replace('-', "_")]);
                        }
                    }
                    Err(_) => {
                        if attempt == MAX_RETRIES {
                            // JSON parsing failed on final attempt, use fallback
                            return Ok(vec![package_name.replace('-', "_")]);
                        }
                        // Retry on JSON parsing error
                    }
                }
            }
            Ok(response) if response.status().is_client_error() => {
                // 4xx errors shouldn't be retried
                return Ok(vec![package_name.replace('-', "_")]);
            }
            _ => {
                if attempt == MAX_RETRIES {
                    // Network request failed on final attempt, use fallback
                    return Ok(vec![package_name.replace('-', "_")]);
                }
                // Retry on network errors
                std::thread::sleep(std::time::Duration::from_millis(100 * (attempt + 1) as u64));
            }
        }
    }

    // Should never reach here due to returns above, but provide fallback
    Ok(vec![package_name.replace('-', "_")])
}

pub mod formatters {
    use super::*;

    pub fn format_text_grouped(result: &ExternalAnalysisResult) -> String {
        let mut output = String::new();
        output.push_str("External Dependencies Analysis:\n\n");

        if result.frequency_analysis.is_empty() {
            output.push_str("No external dependencies found.\n");
            return output;
        }

        output.push_str("=== Frequency Analysis ===\n");

        // Group by usage tiers
        let high_usage: Vec<_> = result
            .frequency_analysis
            .iter()
            .filter(|dep| dep.usage_count >= 30)
            .collect();
        let medium_usage: Vec<_> = result
            .frequency_analysis
            .iter()
            .filter(|dep| dep.usage_count >= 5 && dep.usage_count < 30)
            .collect();
        let low_usage: Vec<_> = result
            .frequency_analysis
            .iter()
            .filter(|dep| dep.usage_count < 5)
            .collect();

        if !high_usage.is_empty() {
            output.push_str("High usage (10+ modules):\n");
            for dep in high_usage {
                output.push_str(&format!(
                    "  {} (used by {} modules)\n",
                    dep.package_name, dep.usage_count
                ));
            }
            output.push('\n');
        }

        if !medium_usage.is_empty() {
            output.push_str("Medium usage (5-9 modules):\n");
            for dep in medium_usage {
                output.push_str(&format!(
                    "  {} (used by {} modules)\n",
                    dep.package_name, dep.usage_count
                ));
            }
            output.push('\n');
        }

        if !low_usage.is_empty() {
            output.push_str("Low usage (1-4 modules):\n");
            for dep in low_usage {
                output.push_str(&format!(
                    "  {} (used by {} modules)\n",
                    dep.package_name, dep.usage_count
                ));
            }
            output.push('\n');
        }

        output.push_str("=== Summary ===\n");
        output.push_str(&format!(
            "Total external packages used: {}\n",
            result.summary.total_used_packages
        ));
        
        if result.declared_externals_count > 0 {
            output.push_str(&format!(
                "Manually declared externals: {}\n",
                result.declared_externals_count
            ));
        }

        // Add undeclared dependencies section
        if !result.undeclared_dependencies.is_empty() {
            output.push_str("\n=== Undeclared Dependencies ===\n");
            output.push_str("(Used in code but not declared in pyproject.toml)\n");
            for dep in &result.undeclared_dependencies {
                output.push_str(&format!("  {}\n", dep));
            }
        }

        // Add unused dependencies section
        if !result.unused_dependencies.is_empty() {
            output.push_str("\n=== Unused Dependencies ===\n");
            output.push_str("(Declared in pyproject.toml but not used in code)\n");
            for dep in &result.unused_dependencies {
                output.push_str(&format!("  {}\n", dep));
            }
        }

        // Add diff summary
        if !result.undeclared_dependencies.is_empty() || !result.unused_dependencies.is_empty() {
            output.push_str("\n=== Dependency Sync Status ===\n");
            output.push_str(&format!(
                "Undeclared dependencies: {}\n",
                result.undeclared_dependencies.len()
            ));
            output.push_str(&format!(
                "Unused dependencies: {}\n",
                result.unused_dependencies.len()
            ));
        } else {
            output.push_str("\n=== Dependency Sync Status ===\n");
            output.push_str("✓ All used dependencies are properly declared in pyproject.toml\n");
            output.push_str("✓ No unused dependencies found\n");
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::DependencyGraph;
    use crate::graph::DependencyType;
    use crate::imports::{ModuleIdentifier, ModuleOrigin};

    fn create_test_module_id(name: &str, origin: ModuleOrigin) -> ModuleIdentifier {
        ModuleIdentifier {
            origin,
            canonical_path: name.to_string(),
        }
    }

    #[test]
    fn test_extract_root_package_name() {
        assert_eq!(extract_root_package_name("numpy"), "numpy");
        assert_eq!(extract_root_package_name("numpy.testing"), "numpy");
        assert_eq!(extract_root_package_name("numpy.testing.utils"), "numpy");
        assert_eq!(extract_root_package_name("scipy.stats"), "scipy");
    }

    #[test]
    fn test_analyze_external_dependencies() {
        use crate::pyproject::{init_for_test, reset_for_test};
        use tempfile::TempDir;

        // Reset parser state to ensure clean test isolation
        reset_for_test();

        // Create a temp directory with no pyproject.toml to ensure clean test state
        let temp_dir = TempDir::new().unwrap();
        init_for_test(temp_dir.path());

        let mut graph = DependencyGraph::new();

        // Add internal modules
        let internal1 = create_test_module_id("myapp.main", ModuleOrigin::Internal);
        let internal2 = create_test_module_id("myapp.utils", ModuleOrigin::Internal);

        // Add external modules
        let numpy_id = create_test_module_id("numpy", ModuleOrigin::External);
        let numpy_testing_id = create_test_module_id("numpy.testing", ModuleOrigin::External);
        let pandas_id = create_test_module_id("pandas", ModuleOrigin::External);

        graph.add_module(internal1.clone());
        graph.add_module(internal2.clone());
        graph.add_module(numpy_id.clone());
        graph.add_module(numpy_testing_id.clone());
        graph.add_module(pandas_id.clone());

        // Add dependencies: internal modules import external modules
        graph
            .add_dependency(&internal1, &numpy_id, DependencyType::Imports)
            .unwrap();
        graph
            .add_dependency(&internal1, &pandas_id, DependencyType::Imports)
            .unwrap();
        graph
            .add_dependency(&internal2, &numpy_testing_id, DependencyType::Imports)
            .unwrap();

        let result = analyze_external_dependencies(&graph).unwrap();

        assert_eq!(result.summary.total_used_packages, 2); // numpy and pandas
        assert_eq!(result.frequency_analysis.len(), 2);

        // Find numpy and pandas in results
        let numpy_usage = result
            .frequency_analysis
            .iter()
            .find(|dep| dep.package_name == "numpy")
            .unwrap();
        let pandas_usage = result
            .frequency_analysis
            .iter()
            .find(|dep| dep.package_name == "pandas")
            .unwrap();

        assert_eq!(numpy_usage.usage_count, 2); // used by both internal1 (directly) and internal2 (via numpy.testing)
        assert_eq!(pandas_usage.usage_count, 1); // used by internal1 only

        // Check that undeclared/unused analysis works correctly
        // This test has no pyproject.toml, so all used dependencies should be undeclared
        assert_eq!(result.undeclared_dependencies.len(), 2); // numpy and pandas
        assert!(
            result
                .undeclared_dependencies
                .contains(&"numpy".to_string())
        );
        assert!(
            result
                .undeclared_dependencies
                .contains(&"pandas".to_string())
        );
        assert!(result.unused_dependencies.is_empty()); // No declared deps means no unused deps
    }

    #[test]
    fn test_stdlib_modules_filtered_out() {
        let mut graph = DependencyGraph::new();

        // Add internal module
        let internal1 = create_test_module_id("myapp.main", ModuleOrigin::Internal);

        // Add external modules - mix of real external and stdlib modules
        let numpy_id = create_test_module_id("numpy", ModuleOrigin::External);
        let sys_id = create_test_module_id("sys", ModuleOrigin::External);
        let os_id = create_test_module_id("os", ModuleOrigin::External);
        let json_id = create_test_module_id("json", ModuleOrigin::External);

        graph.add_module(internal1.clone());
        graph.add_module(numpy_id.clone());
        graph.add_module(sys_id.clone());
        graph.add_module(os_id.clone());
        graph.add_module(json_id.clone());

        // Add dependencies: internal module imports both external and stdlib modules
        graph
            .add_dependency(&internal1, &numpy_id, DependencyType::Imports)
            .unwrap();
        graph
            .add_dependency(&internal1, &sys_id, DependencyType::Imports)
            .unwrap();
        graph
            .add_dependency(&internal1, &os_id, DependencyType::Imports)
            .unwrap();
        graph
            .add_dependency(&internal1, &json_id, DependencyType::Imports)
            .unwrap();

        let result = analyze_external_dependencies(&graph).unwrap();

        // Should only include numpy, not stdlib modules (sys, os, json)
        assert_eq!(result.summary.total_used_packages, 1);
        assert_eq!(result.frequency_analysis.len(), 1);
        assert_eq!(result.frequency_analysis[0].package_name, "numpy");

        // Check that the basic filtering works (stdlib modules excluded)
        // The diff analysis fields should exist (but values depend on global state)
    }

    #[test]
    fn test_used_externals_integration() {
        use crate::pyproject::{init_for_test, reset_for_test};
        use std::fs;
        use tempfile::TempDir;

        // Reset parser state to ensure clean test isolation
        reset_for_test();

        let temp_dir = TempDir::new().unwrap();

        // Create .used-externals.txt with some packages
        let used_externals_content = r#"# Manually declared packages
setuptools
wheel
redis
tensorflow  # This one won't be used in code
"#;
        fs::write(temp_dir.path().join(".used-externals.txt"), used_externals_content).unwrap();

        // Initialize pyproject parser with temp directory
        init_for_test(temp_dir.path());

        let mut graph = DependencyGraph::new();

        // Add internal module
        let internal1 = create_test_module_id("myapp.main", ModuleOrigin::Internal);

        // Add external modules - some declared in .used-externals.txt, some not
        let numpy_id = create_test_module_id("numpy", ModuleOrigin::External); // Code-detected only
        let redis_id = create_test_module_id("redis", ModuleOrigin::External); // Both declared and code-detected
        let setuptools_id = create_test_module_id("setuptools", ModuleOrigin::External); // Declared only (no usage)

        graph.add_module(internal1.clone());
        graph.add_module(numpy_id.clone());
        graph.add_module(redis_id.clone());
        graph.add_module(setuptools_id.clone()); // Add to graph but don't use

        // Add dependencies: use numpy and redis in code
        graph
            .add_dependency(&internal1, &numpy_id, DependencyType::Imports)
            .unwrap();
        graph
            .add_dependency(&internal1, &redis_id, DependencyType::Imports)
            .unwrap();

        let result = analyze_external_dependencies(&graph).unwrap();

        // Should find 5 used packages: 4 from .used-externals.txt + 1 from code (numpy)
        assert_eq!(result.summary.total_used_packages, 5);
        assert_eq!(result.declared_externals_count, 4); // setuptools, wheel, redis, tensorflow

        // Find packages in frequency analysis
        let package_names: Vec<&str> = result
            .frequency_analysis
            .iter()
            .map(|dep| dep.package_name.as_str())
            .collect();

        assert!(package_names.contains(&"numpy")); // Code-detected only
        assert!(package_names.contains(&"redis")); // Both declared and code-detected
        assert!(package_names.contains(&"setuptools")); // Declared only
        assert!(package_names.contains(&"wheel")); // Declared only
        assert!(package_names.contains(&"tensorflow")); // Declared only

        // Check usage counts and sources
        let redis_usage = result
            .frequency_analysis
            .iter()
            .find(|dep| dep.package_name == "redis")
            .unwrap();
        assert_eq!(redis_usage.usage_count, 2); // Both "(declared)" and actual module usage

        let setuptools_usage = result
            .frequency_analysis
            .iter()
            .find(|dep| dep.package_name == "setuptools")
            .unwrap();
        assert_eq!(setuptools_usage.usage_count, 1); // Only "(declared)"
        assert!(setuptools_usage.used_by_modules.contains(&"(declared)".to_string()));

        let numpy_usage = result
            .frequency_analysis
            .iter()
            .find(|dep| dep.package_name == "numpy")
            .unwrap();
        assert_eq!(numpy_usage.usage_count, 1); // Only actual code usage
        assert!(!numpy_usage.used_by_modules.contains(&"(declared)".to_string()));
    }

    #[test]
    fn test_dependency_diff_analysis() {
        use crate::pyproject::{init_for_test, reset_for_test};
        use std::fs;
        use tempfile::TempDir;

        // Reset parser state to ensure clean test isolation
        reset_for_test();

        let temp_dir = TempDir::new().unwrap();

        // Create a mock pyproject.toml with some dependencies
        let pyproject_content = r#"
[tool.poetry.dependencies]
python = ">=3.10,<3.11"
numpy = "^1.24.3"
pandas = "^2.0.3"
unused-package = "^1.0.0"

[tool.poetry.group.dev.dependencies]
pytest = "^7.3.1"
"#;
        fs::write(temp_dir.path().join("pyproject.toml"), pyproject_content).unwrap();

        // Initialize pyproject parser with temp directory (with reset for test isolation)
        init_for_test(temp_dir.path());

        let mut graph = DependencyGraph::new();

        // Add internal module
        let internal1 = create_test_module_id("myapp.main", ModuleOrigin::Internal);

        // Add external modules - some declared, some not
        let numpy_id = create_test_module_id("numpy", ModuleOrigin::External);
        let torch_id = create_test_module_id("torch", ModuleOrigin::External); // undeclared
        let sklearn_id = create_test_module_id("sklearn", ModuleOrigin::External); // undeclared

        graph.add_module(internal1.clone());
        graph.add_module(numpy_id.clone());
        graph.add_module(torch_id.clone());
        graph.add_module(sklearn_id.clone());

        // Add dependencies: use numpy and torch, but not pandas/pytest/unused-package
        graph
            .add_dependency(&internal1, &numpy_id, DependencyType::Imports)
            .unwrap();
        graph
            .add_dependency(&internal1, &torch_id, DependencyType::Imports)
            .unwrap();
        graph
            .add_dependency(&internal1, &sklearn_id, DependencyType::Imports)
            .unwrap();

        let result = analyze_external_dependencies(&graph).unwrap();

        // Should find 3 used packages
        assert_eq!(result.summary.total_used_packages, 3);
        assert_eq!(result.frequency_analysis.len(), 3);

        // Check undeclared dependencies (used but not in pyproject.toml)
        assert!(
            result
                .undeclared_dependencies
                .contains(&"torch".to_string())
        );
        assert!(
            result
                .undeclared_dependencies
                .contains(&"scikit-learn".to_string())
        );
        assert!(
            !result
                .undeclared_dependencies
                .contains(&"numpy".to_string())
        ); // numpy is declared
        assert_eq!(result.undeclared_dependencies.len(), 2);

        // Check unused dependencies (in pyproject.toml but not used)
        assert!(result.unused_dependencies.contains(&"pandas".to_string()));
        assert!(result.unused_dependencies.contains(&"pytest".to_string()));
        assert!(
            result
                .unused_dependencies
                .contains(&"unused-package".to_string())
        );
        assert!(!result.unused_dependencies.contains(&"numpy".to_string())); // numpy is used
        assert_eq!(result.unused_dependencies.len(), 3);
    }

    #[test]
    fn test_get_python_standard_library_modules() {
        let stdlib_modules = get_python_standard_library_modules();

        // Should contain common stdlib modules
        assert!(stdlib_modules.contains("sys"));
        assert!(stdlib_modules.contains("os"));
        assert!(stdlib_modules.contains("json"));
        assert!(stdlib_modules.contains("collections"));

        // Should not contain external packages
        assert!(!stdlib_modules.contains("numpy"));
        assert!(!stdlib_modules.contains("pandas"));
        assert!(!stdlib_modules.contains("torch"));
    }
}
