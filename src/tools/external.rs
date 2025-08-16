use crate::graph::DependencyGraph;
use crate::imports::ModuleOrigin;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ExternalAnalysisResult {
    pub frequency_analysis: Vec<DependencyUsage>,
    pub summary: ExternalDependencySummary,
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

                    package_usage
                        .entry(package_name)
                        .or_insert_with(Vec::new)
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

    let summary = ExternalDependencySummary {
        total_used_packages: frequency_analysis.len(),
    };

    Ok(ExternalAnalysisResult {
        frequency_analysis,
        summary,
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
            .filter(|dep| dep.usage_count >= 10)
            .collect();
        let medium_usage: Vec<_> = result
            .frequency_analysis
            .iter()
            .filter(|dep| dep.usage_count >= 5 && dep.usage_count < 10)
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
    }
}
