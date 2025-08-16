use crate::graph::{DependencyGraph, DependencyType};
use crate::imports::ModuleIdentifier;
use crate::tools::common;
use anyhow::Result;

/// Result of impact analysis for a module
#[derive(Debug)]
pub struct ImpactAnalysisResult {
    /// The module that was analyzed
    pub target_module: String,
    /// Modules affected by changes to the target (deduplicated) with submodule counts
    pub affected_modules: Vec<(String, DependencyType, usize)>,
    /// Total count before deduplication
    pub total_affected_count: usize,
}

pub fn get_impact_analysis(
    graph: &DependencyGraph,
    module_id: &ModuleIdentifier,
) -> Result<(Vec<(String, DependencyType, usize)>, usize)> {
    // Collect dependents of the module and of all its descendants.
    let mut affected_modules = graph.get_transitive_dependents_with_types(module_id)?;

    // Filter out test modules
    affected_modules.retain(|(module_path, _)| {
        !module_path.contains(".tests.") && !module_path.ends_with(".tests")
    });

    // Add parent modules if all their submodules are affected
    let additional_parents =
        find_parent_modules_with_all_children_affected(graph, &affected_modules)?;
    affected_modules.extend(additional_parents);

    let total_count = affected_modules.len();
    let deduplicated = common::filter_hierarchical(affected_modules);

    Ok((deduplicated, total_count))
}

/// Finds parent modules where all their direct children are in the affected list.
/// Returns additional parent modules that should be considered affected.
fn find_parent_modules_with_all_children_affected(
    graph: &DependencyGraph,
    affected_modules: &[(String, DependencyType)],
) -> Result<Vec<(String, DependencyType)>> {
    use std::collections::{HashMap, HashSet};

    // Create a set of affected module names for quick lookup
    let affected_set: HashSet<&String> = affected_modules.iter().map(|(name, _)| name).collect();

    // Build parent-to-children mapping using graph's Contains relationships
    let mut parent_to_children: HashMap<String, Vec<String>> = HashMap::new();

    // Get all modules in the graph and look for Contains relationships
    for parent_module in graph.all_modules() {
        let parent_path = &parent_module.canonical_path;

        // Find all modules that this parent contains
        let children = graph
            .get_dependencies_with_types(parent_module)
            .unwrap_or_else(|_| Vec::new())
            .into_iter()
            .filter_map(|(child_path, dep_type)| {
                if dep_type == DependencyType::Contains {
                    Some(child_path)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if !children.is_empty() {
            parent_to_children.insert(parent_path.clone(), children);
        }
    }

    let mut additional_parents = Vec::new();

    // For each parent, check if all its direct children are affected
    for (parent_path, children) in parent_to_children {
        // Skip if the parent is already in the affected list
        if affected_set.contains(&parent_path) {
            continue;
        }

        // Check if ALL direct children are affected
        if !children.is_empty() && children.iter().all(|child| affected_set.contains(child)) {
            // All children are affected, so the parent should be affected too
            additional_parents.push((parent_path, DependencyType::Imports));
        }
    }

    Ok(additional_parents)
}

/// Analyzes the impact of changes to the specified module
pub fn analyze_impact(graph: &DependencyGraph, module_name: &str) -> Result<ImpactAnalysisResult> {
    // Find the target module in the graph
    let target_module = graph
        .all_modules()
        .find(|m| m.canonical_path == module_name)
        .ok_or_else(|| anyhow::anyhow!("Module '{}' not found in dependency graph", module_name))?;

    // Get impact analysis from the graph
    let (affected_modules, total_count) = get_impact_analysis(&graph, target_module)?;

    Ok(ImpactAnalysisResult {
        target_module: target_module.canonical_path.clone(),
        affected_modules,
        total_affected_count: total_count,
    })
}

/// Formats impact analysis results for display
pub mod formatters {
    use super::ImpactAnalysisResult;
    use crate::tools::common::formatters as common_formatters;

    const NO_DEPENDENCIES_MSG: &str = "(no dependencies found)";

    /// Common formatting structure for all output formats
    fn format_with_body(result: &ImpactAnalysisResult, body: String) -> String {
        format!(
            "Modules depending on '{}':\n{}Total: {} modules impacted by {}\n",
            result.target_module, body, result.total_affected_count, result.target_module
        )
    }

    /// Formats results as human-readable text
    pub fn format_text(result: &ImpactAnalysisResult) -> String {
        let body = if result.affected_modules.is_empty() {
            format!("{}\n", NO_DEPENDENCIES_MSG)
        } else {
            let mut output = String::new();
            for (module, _dep_type, count) in &result.affected_modules {
                if *count > 1 {
                    output.push_str(&format!("({} submodules) {}\n", count, module));
                } else {
                    output.push_str(&format!("{}\n", module));
                }
            }
            output
        };

        format_with_body(result, body)
    }

    /// Formats results with prefix grouping to reduce verbosity
    pub fn format_text_grouped(result: &ImpactAnalysisResult) -> String {
        let body = if result.affected_modules.is_empty() {
            format!("{}\n", NO_DEPENDENCIES_MSG)
        } else {
            common_formatters::format_grouped_modules(&result.affected_modules)
        };

        format_with_body(result, body)
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
    fn test_impact_analyzer_basic() {
        let mut graph = DependencyGraph::new();

        // Set up a simple dependency graph
        let main = create_test_module_id("main", ModuleOrigin::Internal);
        let utils = create_test_module_id("utils", ModuleOrigin::Internal);
        let tests = create_test_module_id("tests.test_utils", ModuleOrigin::Internal);

        graph.add_module(main.clone());
        graph.add_module(utils.clone());
        graph.add_module(tests.clone());

        // main imports utils, tests imports utils
        graph
            .add_dependency(&main, &utils, DependencyType::Imports)
            .unwrap();
        graph
            .add_dependency(&tests, &utils, DependencyType::Imports)
            .unwrap();

        // Analyze impact of utils
        let result = analyze_impact(&graph, "utils").unwrap();

        assert_eq!(result.target_module, "utils");
        assert_eq!(result.affected_modules.len(), 3);
        assert_eq!(result.total_affected_count, 3);

        // Check that utils itself, main, and tests are affected
        let affected_names: Vec<&String> = result
            .affected_modules
            .iter()
            .map(|(name, _, _)| name)
            .collect();
        assert!(affected_names.contains(&&"utils".to_string()));
        assert!(affected_names.contains(&&"main".to_string()));
        assert!(affected_names.contains(&&"tests.test_utils".to_string()));
    }

    #[test]
    fn test_format_text() {
        let result = ImpactAnalysisResult {
            target_module: "utils".to_string(),
            affected_modules: vec![
                ("main".to_string(), DependencyType::Imports, 1),
                ("api".to_string(), DependencyType::Imports, 3),
            ],
            total_affected_count: 4,
        };

        let formatted = formatters::format_text(&result);

        assert!(formatted.contains("Modules depending on 'utils':"));
        assert!(formatted.contains("main"));
        assert!(formatted.contains("(3 submodules) api"));
        assert!(formatted.contains("Total: 4 modules impacted by utils"));
    }
}
