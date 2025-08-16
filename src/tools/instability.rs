use crate::graph::DependencyGraph;
use crate::imports::ModuleOrigin;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};

/// Result of instability analysis
#[derive(Debug)]
pub struct InstabilityAnalysisResult {
    /// Modules with their instability scores (sorted by score descending)
    pub instability_modules: Vec<(String, f64)>,
}

/// Analyzes instability in the codebase - modules with the highest instability scores
///
/// Instability (I) = Ce / (Ca + Ce) where:
/// - Ce (Efferent Coupling): Number of modules this module depends on
/// - Ca (Afferent Coupling): Number of modules that depend on this module
/// - Range: 0.0 (stable) to 1.0 (unstable)
pub fn analyze_instability(graph: &DependencyGraph) -> Result<InstabilityAnalysisResult> {
    let mut instability_modules = Vec::new();

    // Collect internal modules for analysis
    let internal_modules: Vec<_> = graph
        .all_modules()
        .filter(|module| module.origin == ModuleOrigin::Internal)
        .collect();

    if internal_modules.is_empty() {
        return Ok(InstabilityAnalysisResult {
            instability_modules,
        });
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
    pb.set_message("Analyzing instability metrics");

    // Iterate through all internal modules and calculate their instability scores
    for module in internal_modules {
        pb.set_message(format!("Analyzing {}", module.canonical_path));

        // Calculate afferent coupling (Ca) - modules that depend on this module
        let dependents = graph.get_dependents(module)?;
        let ca = dependents.len();

        // Calculate efferent coupling (Ce) - modules this module depends on
        let dependencies = graph.get_dependencies(module)?;
        let ce = dependencies.len();

        // Calculate instability: Ce / (Ca + Ce)
        // If both Ca and Ce are 0, treat as stable (instability = 0.0)
        let instability = if ca + ce == 0 {
            0.0
        } else {
            ce as f64 / (ca + ce) as f64
        };

        instability_modules.push((module.canonical_path.clone(), instability));

        pb.inc(1);
    }

    pb.finish_with_message("Instability analysis complete");

    // Sort by instability score (descending) - highest instability first
    instability_modules.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    Ok(InstabilityAnalysisResult {
        instability_modules,
    })
}

/// Formats instability analysis results for display
pub mod formatters {
    use super::InstabilityAnalysisResult;

    /// Formats results as human-readable text
    pub fn format_text(result: &InstabilityAnalysisResult) -> String {
        if result.instability_modules.is_empty() {
            return "No modules found.\n".to_string();
        }

        let mut output = String::from("High-instability modules (most unstable first):\n");
        for (module, instability) in &result.instability_modules {
            output.push_str(&format!("  {} (instability: {:.3})\n", module, instability));
        }
        output.push_str(&format!(
            "\nTotal: {} modules found\n",
            result.instability_modules.len()
        ));
        output
    }
}
