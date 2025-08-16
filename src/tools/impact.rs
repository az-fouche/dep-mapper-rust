use crate::graph::{DependencyGraph, DependencyType};
use crate::imports::ModuleIdentifier;
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
    affected_modules.retain(|(module_path, _)| !module_path.contains(".tests.") && !module_path.ends_with(".tests"));

    // Add parent modules if all their submodules are affected
    let additional_parents = find_parent_modules_with_all_children_affected(graph, &affected_modules)?;
    affected_modules.extend(additional_parents);

    let total_count = affected_modules.len();
    let deduplicated = filter_hierarchical(affected_modules);

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
        let children = graph.get_dependencies_with_types(parent_module)
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

/// Deduplicates a list of modules by removing children when their parent is present,
/// and tracks how many original modules each deduplicated entry represents.
fn filter_hierarchical(
    mut modules: Vec<(String, DependencyType)>,
) -> Vec<(String, DependencyType, usize)> {
    use std::collections::HashMap;

    // First, deduplicate exact module names, keeping first dependency type
    let mut seen_modules = HashMap::new();
    modules.retain(|(module_path, dep_type)| {
        if seen_modules.contains_key(module_path) {
            false
        } else {
            seen_modules.insert(module_path.clone(), dep_type.clone());
            true
        }
    });

    // Sort by path to ensure consistent processing
    modules.sort_by(|a, b| a.0.cmp(&b.0));

    let mut result = Vec::new();

    for (module_path, dep_type) in modules {
        // Check if any module already in result is a parent of this module
        let parent_index = result
            .iter()
            .position(|(existing_path, _, _): &(String, DependencyType, usize)| {
                module_path.starts_with(&format!("{}.", existing_path))
            });

        if let Some(index) = parent_index {
            // This module is a child of an existing parent, increment the parent's count
            result[index].2 += 1;
        } else {
            // Count how many existing modules are children of this module
            let mut child_count = 1; // Count self
            let mut indices_to_remove = Vec::new();
            
            for (i, (existing_path, _, existing_count)) in result.iter().enumerate() {
                if existing_path.starts_with(&format!("{}.", module_path)) {
                    child_count += existing_count;
                    indices_to_remove.push(i);
                }
            }

            // Remove children in reverse order to maintain indices
            for &i in indices_to_remove.iter().rev() {
                result.remove(i);
            }

            result.push((module_path, dep_type, child_count));
        }
    }

    result
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

    /// Formats results as human-readable text
    pub fn format_text(result: &ImpactAnalysisResult) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "Modules depending on '{}':\n",
            result.target_module
        ));

        if result.affected_modules.is_empty() {
            output.push_str("(no dependencies found)\n");
        } else {
            for (module, _dep_type, count) in &result.affected_modules {
                if *count > 1 {
                    output.push_str(&format!("({} submodules) {}\n", count, module));
                } else {
                    output.push_str(&format!("{}\n", module));
                }
            }
        }

        output.push_str(&format!(
            "Total: {} modules impacted by {}\n",
            result.total_affected_count, 
            result.target_module
        ));

        output
    }

    /// Formats results with prefix grouping to reduce verbosity
    pub fn format_text_grouped(result: &ImpactAnalysisResult) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "Modules depending on '{}':\n",
            result.target_module
        ));

        if result.affected_modules.is_empty() {
            output.push_str("(no dependencies found)\n");
        } else {
            output.push_str(&format_grouped_modules(&result.affected_modules));
        }

        output.push_str(&format!(
            "Total: {} modules affected\n",
            result.total_affected_count
        ));

        output
    }

    fn format_grouped_modules(modules: &[(String, super::DependencyType, usize)]) -> String {
        use std::collections::HashMap;
        
        let mut output = String::new();
        let mut current_prefix: Vec<String> = Vec::new();
        
        // Pre-calculate counts for all path prefixes
        let mut prefix_counts: HashMap<String, usize> = HashMap::new();
        for (module_path, _dep_type, count) in modules {
            let segments: Vec<&str> = module_path.split('.').collect();
            for i in 1..segments.len() {
                let prefix = segments[0..i].join(".");
                *prefix_counts.entry(prefix).or_insert(0) += count;
            }
        }

        for (module_path, _dep_type, count) in modules {
            let segments: Vec<String> = module_path.split('.').map(|s| s.to_string()).collect();
            
            // Find common prefix length
            let common_len = current_prefix
                .iter()
                .zip(segments.iter())
                .take_while(|(a, b)| a == b)
                .count();
            
            // Output the new segments that differ from current prefix
            for (i, segment) in segments.iter().enumerate() {
                if i < common_len {
                    continue; // Skip common prefix parts
                }
                
                let indent = "  ".repeat(i);
                let prefix_char = if i > 0 { "." } else { "" };
                
                if i == segments.len() - 1 {
                    // This is the final segment - show count if > 1
                    if *count > 1 {
                        output.push_str(&format!("{}{}{} ({})\n", indent, prefix_char, segment, count));
                    } else {
                        output.push_str(&format!("{}{}{}\n", indent, prefix_char, segment));
                    }
                } else {
                    // This is an intermediate segment - show count if it has multiple children
                    let current_path = segments[0..=i].join(".");
                    if let Some(&prefix_count) = prefix_counts.get(&current_path) {
                        if prefix_count > 1 {
                            output.push_str(&format!("{}{}{} ({})\n", indent, prefix_char, segment, prefix_count));
                        } else {
                            output.push_str(&format!("{}{}{}\n", indent, prefix_char, segment));
                        }
                    } else {
                        output.push_str(&format!("{}{}{}\n", indent, prefix_char, segment));
                    }
                }
            }
            
            // Update current prefix to this module's segments
            current_prefix = segments;
        }

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
