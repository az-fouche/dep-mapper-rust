use crate::graph::DependencyGraph;
use crate::tools::cycles::{detect_cycles, Cycle};
use crate::tools::external::analyze_external_dependencies;
use crate::tools::instability::analyze_instability;
use crate::tools::pressure::analyze_pressure;
use anyhow::Result;

/// Raw data from diagnose analysis - no display logic
#[derive(Debug)]
pub struct DiagnoseResult {
    /// Total number of modules analyzed
    pub total_modules: usize,
    /// Number of circular dependencies found
    pub cycle_count: usize,
    /// Top 5 longest cycles (sorted by length descending)
    pub top_cycles: Vec<Cycle>,
    /// Average instability score across all modules
    pub avg_instability: f64,
    /// Instability quantiles (10%, 50%, 90%)
    pub instability_quantiles: (f64, f64, f64),
    /// Number of modules by pressure levels (>10, >50, >100 dependents)
    pub pressure_levels: (usize, usize, usize),
    /// Number of external dependencies
    pub external_dependency_count: usize,
}

/// Performs comprehensive diagnosis of the codebase
pub fn analyze_diagnose(graph: &DependencyGraph) -> Result<DiagnoseResult> {
    // Get basic graph metrics
    let total_modules = graph.all_modules().count();
    
    // Run existing analyses
    let cycles_result = detect_cycles(graph)?;
    let cycle_count = cycles_result.cycles.len();
    
    // Get top 5 longest cycles (sorted by length descending)
    let mut cycles_by_length = cycles_result.cycles.clone();
    cycles_by_length.sort_by(|a, b| b.modules.len().cmp(&a.modules.len()));
    let top_cycles = cycles_by_length.into_iter().take(5).collect();
    
    let instability_result = analyze_instability(graph)?;
    let avg_instability = if instability_result.instability_modules.is_empty() {
        0.0
    } else {
        instability_result.instability_modules
            .iter()
            .map(|(_, score)| score)
            .sum::<f64>() / instability_result.instability_modules.len() as f64
    };
    
    let pressure_result = analyze_pressure(graph)?;
    
    // Calculate pressure levels (>10, >50, >100 dependents)
    let pressure_over_10 = pressure_result.pressure_modules
        .iter()
        .filter(|(_, count)| *count > 10)
        .count();
    let pressure_over_50 = pressure_result.pressure_modules
        .iter()
        .filter(|(_, count)| *count > 50)
        .count();
    let pressure_over_100 = pressure_result.pressure_modules
        .iter()
        .filter(|(_, count)| *count > 100)
        .count();
    let pressure_levels = (pressure_over_10, pressure_over_50, pressure_over_100);
    
    // Calculate instability quantiles (10%, 50%, 90%)
    let instability_quantiles = calculate_instability_quantiles(&instability_result.instability_modules);
    
    let external_result = analyze_external_dependencies(graph)?;
    let external_dependency_count = external_result.frequency_analysis.len();
    
    Ok(DiagnoseResult {
        total_modules,
        cycle_count,
        top_cycles,
        avg_instability,
        instability_quantiles,
        pressure_levels,
        external_dependency_count,
    })
}

/// Calculate instability quantiles (10%, 50%, 90%)
fn calculate_instability_quantiles(instability_modules: &[(String, f64)]) -> (f64, f64, f64) {
    if instability_modules.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    
    // Extract and sort the instability scores
    let mut scores: Vec<f64> = instability_modules.iter().map(|(_, score)| *score).collect();
    scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let len = scores.len();
    let q10_idx = (len as f64 * 0.1) as usize;
    let q50_idx = (len as f64 * 0.5) as usize;
    let q90_idx = (len as f64 * 0.9) as usize;
    
    // Ensure indices are within bounds
    let q10_idx = q10_idx.min(len - 1);
    let q50_idx = q50_idx.min(len - 1);
    let q90_idx = q90_idx.min(len - 1);
    
    (scores[q10_idx], scores[q50_idx], scores[q90_idx])
}

/// Formatters for diagnose results
pub mod formatters {
    use super::{DiagnoseResult, Cycle};

    /// Formats results as human-readable text
    pub fn format_text(result: &DiagnoseResult) -> String {
        let (q10, q50, q90) = result.instability_quantiles;
        let (pressure_10, pressure_50, pressure_100) = result.pressure_levels;
        
        format!(
            "CODEBASE ARCHITECTURE METRICS\n\
             ==============================\n\n\
             📊 OVERVIEW\n\
             -----------\n\
             Total Modules: {}\n\
             External Dependencies: {}\n\n\
             🔄 CIRCULAR DEPENDENCIES\n\
             ------------------------\n\
             Count: {}\n\
             {}\n\
             {}\n\n\
             ⚖️ INSTABILITY ANALYSIS\n\
             -----------------------\n\
             Average: {:.3}\n\
             10th percentile: {:.3}\n\
             50th percentile (median): {:.3}\n\
             90th percentile: {:.3}\n\
             {}\n\n\
             🔥 PRESSURE POINTS\n\
             ------------------\n\
             Modules with >10 dependents: {}\n\
             Modules with >50 dependents: {}\n\
             Modules with >100 dependents: {}\n\
             {}\n",
            result.total_modules,
            result.external_dependency_count,
            result.cycle_count,
            if result.cycle_count > 0 { 
                "⚠️ Circular dependencies found - consider refactoring" 
            } else { 
                "✅ No circular dependencies detected" 
            },
            format_top_cycles(&result.top_cycles),
            result.avg_instability,
            q10, q50, q90,
            if result.avg_instability > 0.5 { 
                "⚠️ High average instability - modules are highly coupled" 
            } else { 
                "✅ Reasonable instability levels" 
            },
            pressure_10, pressure_50, pressure_100,
            if pressure_10 > 0 { 
                "⚠️ High-pressure modules found - consider splitting large dependencies" 
            } else { 
                "✅ No high-pressure modules detected" 
            }
        )
    }
    
    /// Format the top cycles for display
    fn format_top_cycles(cycles: &[Cycle]) -> String {
        if cycles.is_empty() {
            return String::new();
        }
        
        let mut output = String::from("Top cycles by length:\n");
        for (i, cycle) in cycles.iter().enumerate() {
            output.push_str(&format!(
                "  {}. {} (length: {})\n", 
                i + 1, 
                cycle.format_cycle(),
                cycle.modules.len()
            ));
        }
        output
    }
}