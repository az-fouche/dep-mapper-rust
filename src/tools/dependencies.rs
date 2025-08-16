use crate::graph::{DependencyGraph, DependencyType};
use crate::imports::{ModuleIdentifier, ModuleOrigin};
use crate::tools::common;
use anyhow::Result;

/// Result of dependency analysis for a module
#[derive(Debug)]
pub struct DependencyAnalysisResult {
    /// The module that was analyzed
    pub target_module: String,
    /// External package dependencies
    pub external_dependencies: Vec<String>,
    /// Internal module dependencies with hierarchy info
    pub internal_dependencies: Vec<(String, DependencyType, usize)>,
    /// Total count of dependencies
    pub total_dependency_count: usize,
}

pub fn get_dependencies_analysis(
    graph: &DependencyGraph,
    module_id: &ModuleIdentifier,
) -> Result<(Vec<String>, Vec<(String, DependencyType, usize)>, usize)> {
    // Collect dependencies of the module and of all its descendants.
    let mut all_dependencies = graph.get_transitive_dependencies_with_types(module_id)?;

    // Filter out test modules
    all_dependencies.retain(|(module_path, _)| {
        !module_path.contains(".tests.") && !module_path.ends_with(".tests")
    });

    // Separate external and internal dependencies
    let mut external_dependencies = Vec::new();
    let mut internal_raw_dependencies = Vec::new();

    for (dep_path, dep_type) in all_dependencies {
        // Check if this dependency is external by looking it up in the graph
        let is_external = graph
            .all_modules()
            .find(|m| m.canonical_path == dep_path)
            .map(|m| m.origin == ModuleOrigin::External)
            .unwrap_or(true); // If not found in graph, assume external

        if is_external {
            external_dependencies.push(dep_path);
        } else {
            internal_raw_dependencies.push((dep_path, dep_type));
        }
    }

    let total_count = external_dependencies.len() + internal_raw_dependencies.len();
    let deduplicated_internal = common::filter_hierarchical(internal_raw_dependencies);

    // Sort external dependencies for consistent output
    external_dependencies.sort();
    external_dependencies.dedup();

    Ok((external_dependencies, deduplicated_internal, total_count))
}

/// Analyzes the dependencies of the specified module
pub fn analyze_dependencies(
    graph: &DependencyGraph,
    module_name: &str,
) -> Result<DependencyAnalysisResult> {
    // Find the target module in the graph
    let target_module = graph
        .all_modules()
        .find(|m| m.canonical_path == module_name)
        .ok_or_else(|| anyhow::anyhow!("Module '{}' not found in dependency graph", module_name))?;

    // Get dependencies analysis from the graph
    let (external_dependencies, internal_dependencies, total_count) =
        get_dependencies_analysis(graph, target_module)?;

    Ok(DependencyAnalysisResult {
        target_module: target_module.canonical_path.clone(),
        external_dependencies,
        internal_dependencies,
        total_dependency_count: total_count,
    })
}

/// Formats dependency analysis results for display
pub mod formatters {
    use super::DependencyAnalysisResult;
    use crate::tools::common::formatters as common_formatters;

    const NO_DEPENDENCIES_MSG: &str = "(no dependencies found)";

    /// Formats results as human-readable text
    pub fn format_text(result: &DependencyAnalysisResult) -> String {
        let mut output = format!("Dependencies of '{}':\n", result.target_module);

        if result.external_dependencies.is_empty() && result.internal_dependencies.is_empty() {
            output.push_str(&format!("{}\n", NO_DEPENDENCIES_MSG));
        } else {
            // External dependencies section
            if !result.external_dependencies.is_empty() {
                output.push_str("External packages:\n");
                for dep in &result.external_dependencies {
                    output.push_str(&format!("  {}\n", dep));
                }
            }

            // Internal dependencies section
            if !result.internal_dependencies.is_empty() {
                if !result.external_dependencies.is_empty() {
                    output.push('\n');
                }
                output.push_str("Internal modules:\n");
                for (module, _dep_type, count) in &result.internal_dependencies {
                    if *count > 1 {
                        output.push_str(&format!("  ({} submodules) {}\n", count, module));
                    } else {
                        output.push_str(&format!("  {}\n", module));
                    }
                }
            }
        }

        output.push_str(&format!(
            "Total: {} dependencies ({} external, {} internal)\n",
            result.total_dependency_count,
            result.external_dependencies.len(),
            result.internal_dependencies.len()
        ));

        output
    }

    /// Formats results with prefix grouping to reduce verbosity for internal modules
    pub fn format_text_grouped(result: &DependencyAnalysisResult) -> String {
        let mut output = format!("Dependencies of '{}':\n", result.target_module);

        if result.external_dependencies.is_empty() && result.internal_dependencies.is_empty() {
            output.push_str(&format!("{}\n", NO_DEPENDENCIES_MSG));
        } else {
            // External dependencies section (always shown flat)
            if !result.external_dependencies.is_empty() {
                output.push_str("External packages:\n");
                for dep in &result.external_dependencies {
                    output.push_str(&format!("  {}\n", dep));
                }
            }

            // Internal dependencies section with grouping
            if !result.internal_dependencies.is_empty() {
                if !result.external_dependencies.is_empty() {
                    output.push('\n');
                }
                output.push_str("Internal modules:\n");
                output.push_str(&common_formatters::format_grouped_modules(
                    &result.internal_dependencies,
                ));
            }
        }

        output.push_str(&format!(
            "Total: {} dependencies ({} external, {} internal)\n",
            result.total_dependency_count,
            result.external_dependencies.len(),
            result.internal_dependencies.len()
        ));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::DependencyGraph;
    use crate::imports::{ModuleIdentifier, ModuleOrigin};

    fn create_test_module_id(name: &str, origin: ModuleOrigin) -> ModuleIdentifier {
        ModuleIdentifier {
            origin,
            canonical_path: name.to_string(),
        }
    }

    #[test]
    fn test_dependencies_analyzer_basic() {
        let mut graph = DependencyGraph::new();

        // Set up a simple dependency graph
        let main = create_test_module_id("main", ModuleOrigin::Internal);
        let utils = create_test_module_id("utils", ModuleOrigin::Internal);
        let numpy = create_test_module_id("numpy", ModuleOrigin::External);
        let pandas = create_test_module_id("pandas", ModuleOrigin::External);

        graph.add_module(main.clone());
        graph.add_module(utils.clone());
        graph.add_module(numpy.clone());
        graph.add_module(pandas.clone());

        // main imports utils, numpy, and pandas
        graph
            .add_dependency(&main, &utils, DependencyType::Imports)
            .unwrap();
        graph
            .add_dependency(&main, &numpy, DependencyType::Imports)
            .unwrap();
        graph
            .add_dependency(&main, &pandas, DependencyType::Imports)
            .unwrap();

        // Analyze dependencies of main
        let result = analyze_dependencies(&graph, "main").unwrap();

        assert_eq!(result.target_module, "main");
        assert_eq!(result.external_dependencies.len(), 2);
        assert_eq!(result.internal_dependencies.len(), 1);
        assert_eq!(result.total_dependency_count, 3);

        // Check external dependencies
        assert!(result.external_dependencies.contains(&"numpy".to_string()));
        assert!(result.external_dependencies.contains(&"pandas".to_string()));

        // Check internal dependencies
        let internal_names: Vec<&String> = result
            .internal_dependencies
            .iter()
            .map(|(name, _, _)| name)
            .collect();
        assert!(internal_names.contains(&&"utils".to_string()));
    }

    #[test]
    fn test_format_text() {
        let result = DependencyAnalysisResult {
            target_module: "main".to_string(),
            external_dependencies: vec!["numpy".to_string(), "pandas".to_string()],
            internal_dependencies: vec![
                ("utils".to_string(), DependencyType::Imports, 1),
                ("api".to_string(), DependencyType::Imports, 3),
            ],
            total_dependency_count: 4,
        };

        let formatted = formatters::format_text(&result);

        assert!(formatted.contains("Dependencies of 'main':"));
        assert!(formatted.contains("External packages:"));
        assert!(formatted.contains("numpy"));
        assert!(formatted.contains("pandas"));
        assert!(formatted.contains("Internal modules:"));
        assert!(formatted.contains("utils"));
        assert!(formatted.contains("(3 submodules) api"));
        assert!(formatted.contains("Total: 4 dependencies (2 external, 2 internal)"));
    }

    #[test]
    fn test_no_dependencies() {
        let result = DependencyAnalysisResult {
            target_module: "isolated".to_string(),
            external_dependencies: vec![],
            internal_dependencies: vec![],
            total_dependency_count: 0,
        };

        let formatted = formatters::format_text(&result);

        assert!(formatted.contains("Dependencies of 'isolated':"));
        assert!(formatted.contains("(no dependencies found)"));
        assert!(formatted.contains("Total: 0 dependencies (0 external, 0 internal)"));
    }
}
