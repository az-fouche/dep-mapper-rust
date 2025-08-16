use crate::graph::DependencyGraph;
use crate::imports::ModuleOrigin;
use crate::tools::impact::get_impact_analysis;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};

/// Result of pressure points analysis
#[derive(Debug)]
pub struct PressureAnalysisResult {
    /// Modules with their dependent counts (sorted by count descending)
    pub pressure_modules: Vec<(String, usize)>,
}

/// Analyzes pressure points in the codebase - modules with the most dependents
pub fn analyze_pressure(graph: &DependencyGraph) -> Result<PressureAnalysisResult> {
    let mut pressure_modules = Vec::new();

    // Collect internal modules for analysis
    let internal_modules: Vec<_> = graph
        .all_modules()
        .filter(|module| module.origin == ModuleOrigin::Internal)
        .collect();

    if internal_modules.is_empty() {
        return Ok(PressureAnalysisResult { pressure_modules });
    }

    // Set up progress bar
    let pb = ProgressBar::new(internal_modules.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}",
            )
            .map_err(|e| anyhow::anyhow!("Failed to set progress bar style: {}", e))?
            .progress_chars("##-"),
    );
    pb.set_message("Analyzing pressure points");

    // Iterate through all internal modules and get their dependent counts
    for module in internal_modules {
        pb.set_message(format!("Analyzing {}", module.canonical_path));

        let (affected_modules, _) = get_impact_analysis(graph, module)?;
        let dependent_count = affected_modules.len();

        // Only include modules that have more than 1 dependent (exclude self-only dependencies)
        if dependent_count > 1 {
            pressure_modules.push((module.canonical_path.clone(), dependent_count));
        }

        pb.inc(1);
    }

    pb.finish_with_message("Pressure analysis complete");

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
        output.push_str(&format!(
            "\nTotal: {} modules found\n",
            result.pressure_modules.len()
        ));
        output
    }
}
