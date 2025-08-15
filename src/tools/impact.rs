use crate::graph::{DependencyGraph, DependencyType};
use crate::imports::ModuleIdentifier;
use anyhow::Result;

/// Result of impact analysis for a module
#[derive(Debug)]
pub struct ImpactAnalysisResult {
    /// The module that was analyzed
    pub target_module: String,
    /// Modules affected by changes to the target (deduplicated)
    pub affected_modules: Vec<(String, DependencyType)>,
    /// Total count before deduplication
    pub total_affected_count: usize,
}

pub fn get_impact_analysis(
    graph: &DependencyGraph,
    module_id: &ModuleIdentifier,
) -> Result<(Vec<(String, DependencyType)>, usize)> {
    // Collect dependents of the module and of all its descendants.
    let affected_modules = graph.get_transitive_dependents_with_types(module_id)?;
    // If you want the "raw" count prior to de-dup across descendants,
    // switch to a variant that doesn't de-dup. Here it's already de-duped.
    let total_count = affected_modules.len();
    let deduplicated = deduplicate_hierarchical_modules(affected_modules);

    Ok((deduplicated, total_count))
}

/// Deduplicates a list of modules by removing children when their parent is present.
fn deduplicate_hierarchical_modules(mut modules: Vec<(String, DependencyType)>) -> Vec<(String, DependencyType)> {
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
        let is_child_of_existing = result.iter().any(|(existing_path, _): &(String, DependencyType)| {
            module_path.starts_with(&format!("{}.", existing_path))
        });
        
        if !is_child_of_existing {
            // Remove any existing modules that are children of this module
            result.retain(|(existing_path, _): &(String, DependencyType)| {
                !existing_path.starts_with(&format!("{}.", module_path))
            });
            
            result.push((module_path, dep_type));
        }
    }
    
    result
}

/// Analyzes the impact of changes to the specified module
pub fn analyze_impact(
    graph: &DependencyGraph,
    module_name: &str,
) -> Result<ImpactAnalysisResult> {
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
        
        output.push_str(&format!("Modules depending on '{}':\n", result.target_module));

        if result.affected_modules.is_empty() {
            output.push_str("(no dependencies found)\n");
        } else {
            for (module, dep_type) in &result.affected_modules {
                output.push_str(&format!("- {} ({:?})\n", module, dep_type));
            }
        }

        output.push_str(&format!("Total: {} modules affected\n", result.total_affected_count));
        
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
        graph.add_dependency(&main, &utils, DependencyType::Imports).unwrap();
        graph.add_dependency(&tests, &utils, DependencyType::Imports).unwrap();

        // Analyze impact of utils
        let result = analyze_impact(&graph, "utils").unwrap();

        assert_eq!(result.target_module, "utils");
        assert_eq!(result.affected_modules.len(), 2);
        assert_eq!(result.total_affected_count, 2);

        // Check that both main and tests are affected
        let affected_names: Vec<&String> = result.affected_modules.iter().map(|(name, _)| name).collect();
        assert!(affected_names.contains(&&"main".to_string()));
        assert!(affected_names.contains(&&"tests.test_utils".to_string()));
    }


    #[test]
    fn test_format_text() {
        let result = ImpactAnalysisResult {
            target_module: "utils".to_string(),
            affected_modules: vec![
                ("main".to_string(), DependencyType::Imports),
                ("api".to_string(), DependencyType::Imports),
            ],
            total_affected_count: 2,
        };

        let formatted = formatters::format_text(&result);
        
        assert!(formatted.contains("Modules depending on 'utils':"));
        assert!(formatted.contains("- main (Imports)"));
        assert!(formatted.contains("- api (Imports)"));
        assert!(formatted.contains("Total: 2 modules affected"));
    }
}