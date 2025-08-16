use crate::graph::DependencyGraph;
use crate::imports::ModuleOrigin;
use crate::tools::impact::get_impact_analysis;
use anyhow::Result;

/// Result of pressure points analysis
#[derive(Debug)]
pub struct PressureAnalysisResult {
    /// Modules with their dependent counts (sorted by count descending)
    pub pressure_modules: Vec<(String, usize)>,
}

/// Analyzes pressure points in the codebase - modules with the most dependents
pub fn analyze_pressure(graph: &DependencyGraph) -> Result<PressureAnalysisResult> {
    let mut pressure_modules = Vec::new();

    // Iterate through all modules and get their dependent counts
    for module in graph.all_modules() {
        // Only analyze internal modules
        if module.origin != ModuleOrigin::Internal {
            continue;
        }

        let (affected_modules, _) = get_impact_analysis(graph, module)?;
        let dependent_count = affected_modules.len();
        
        // Only include modules that have more than 1 dependent (exclude self-only dependencies)
        if dependent_count > 1 {
            pressure_modules.push((module.canonical_path.clone(), dependent_count));
        }
    }

    // Sort by dependent count (descending) - highest pressure first
    pressure_modules.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(PressureAnalysisResult { pressure_modules })
}

/// Formats pressure analysis results for display
pub mod formatters {
    use super::PressureAnalysisResult;

    /// Formats results as human-readable text
    pub fn format_text(result: &PressureAnalysisResult) -> String {
        if result.pressure_modules.is_empty() {
            return "No modules with dependents found.\n".to_string();
        }

        let mut output = String::from("High-pressure modules (most dependents first):\n");
        for (module, count) in &result.pressure_modules {
            output.push_str(&format!("  {} ({} dependents)\n", module, count));
        }
        output.push_str(&format!("\nTotal: {} modules found\n", result.pressure_modules.len()));
        output
    }
}