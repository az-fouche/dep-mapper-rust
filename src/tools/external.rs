use crate::graph::DependencyGraph;
use crate::imports::ModuleOrigin;
use crate::pyproject;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct ExternalAnalysisResult {
    pub frequency_analysis: Vec<DependencyUsage>,
    pub summary: ExternalDependencySummary,
    pub undeclared_dependencies: Vec<String>,
    pub unused_dependencies: Vec<String>,
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
    let stdlib_modules = get_python_standard_library_modules();
    let mut package_usage: HashMap<String, Vec<String>> = HashMap::new();

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

    // Get declared dependencies from pyproject.toml
    let declared_deps: HashSet<String> = pyproject::get_declared_dependencies()?
        .into_iter()
        .collect();

    // Get actually used dependencies
    let used_deps: HashSet<String> = frequency_analysis
        .iter()
        .map(|dep| dep.package_name.clone())
        .collect();

    // Find undeclared dependencies (used but not declared in pyproject.toml)
    let mut undeclared_dependencies: Vec<String> =
        used_deps.difference(&declared_deps).cloned().collect();
    undeclared_dependencies.sort();

    // Find unused dependencies (declared but not used)
    let mut unused_dependencies: Vec<String> =
        declared_deps.difference(&used_deps).cloned().collect();
    unused_dependencies.sort();

    let summary = ExternalDependencySummary {
        total_used_packages: frequency_analysis.len(),
    };

    Ok(ExternalAnalysisResult {
        frequency_analysis,
        summary,
        undeclared_dependencies,
        unused_dependencies,
    })
}

/// Cached Python standard library modules
static PYTHON_STDLIB_MODULES: OnceLock<HashSet<String>> = OnceLock::new();

/// Get Python standard library modules by calling Python subprocess
fn get_python_standard_library_modules() -> &'static HashSet<String> {
    PYTHON_STDLIB_MODULES.get_or_init(|| {
        // Try to call Python to get stdlib modules
        match Command::new("python3")
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

                // Debug output for tests
                #[cfg(test)]
                eprintln!("Python3 call succeeded, got {} modules", result.len());

                result
            }
            Ok(_output) => {
                #[cfg(test)]
                eprintln!("Python3 call failed with exit code: {:?}", _output.status);

                // Fallback: try python instead of python3
                match Command::new("python")
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

                        #[cfg(test)]
                        eprintln!("Python call succeeded, got {} modules", result.len());

                        result
                    }
                    Ok(_output) => {
                        #[cfg(test)]
                        eprintln!("Python call failed with exit code: {:?}", _output.status);

                        // If Python call fails, return empty set
                        HashSet::new()
                    }
                    Err(_e2) => {
                        #[cfg(test)]
                        eprintln!("Python call also failed: {}", _e2);

                        // If Python call fails, return empty set
                        HashSet::new()
                    }
                }
            }
            Err(_e) => {
                #[cfg(test)]
                eprintln!("Python3 command failed: {}", _e);

                // If Python call fails, return empty set
                HashSet::new()
            }
        }
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
                .contains(&"sklearn".to_string())
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
