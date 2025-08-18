use crate::graph::DependencyGraph;
use crate::imports::{ModuleIdentifier, ModuleOrigin};
use anyhow::Result;
use std::collections::{HashMap, HashSet};

/// Scope of changeset analysis
#[derive(Debug, Clone)]
pub enum ChangesetScope {
    /// Show what would be affected if the module changes
    Affected,
    /// Show what the module depends on
    Dependencies,
    /// Show both affected modules and dependencies
    Both,
}

impl ChangesetScope {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "affected" => ChangesetScope::Affected,
            "dependencies" => ChangesetScope::Dependencies,
            "both" => ChangesetScope::Both,
            _ => ChangesetScope::Both, // Default to both
        }
    }
}

/// Risk level for modules in a changeset
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RiskLevel {
    /// Low risk - few dependents, well isolated
    Low,
    /// Medium risk - moderate coupling
    Medium,
    /// High risk - many dependents or critical paths
    High,
    /// Critical risk - core infrastructure modules
    Critical,
}

/// A module in the changeset with its risk assessment
#[derive(Debug, Clone)]
pub struct ChangesetModule {
    pub module_name: String,
    pub risk_level: RiskLevel,
    pub dependent_count: usize,
    pub dependency_depth: usize,
    pub is_external: bool,
}

/// Raw data from changeset analysis
#[derive(Debug)]
pub struct ChangesetResult {
    /// Target module being analyzed
    pub target_module: String,
    /// Scope of the analysis
    pub scope: ChangesetScope,
    /// Modules that would be affected by changes to target
    pub affected_modules: Vec<ChangesetModule>,
    /// Modules that target depends on
    pub dependency_modules: Vec<ChangesetModule>,
    /// Suggested test execution order (lowest to highest risk)
    pub test_order: Vec<String>,
    /// Summary statistics
    pub total_affected: usize,
    pub total_dependencies: usize,
    pub high_risk_count: usize,
}

/// Performs changeset analysis on a module
pub fn analyze_changeset(
    graph: &DependencyGraph,
    module_name: &str,
    scope: ChangesetScope,
) -> Result<ChangesetResult> {
    // Find the target module
    let target_module_id = find_module_by_name(graph, module_name)?;

    let mut affected_modules = Vec::new();
    let mut dependency_modules = Vec::new();

    // Analyze affected modules (what breaks if target changes)
    if matches!(scope, ChangesetScope::Affected | ChangesetScope::Both) {
        affected_modules = analyze_affected_modules(graph, &target_module_id)?;
    }

    // Analyze dependencies (what target needs)
    if matches!(scope, ChangesetScope::Dependencies | ChangesetScope::Both) {
        dependency_modules = analyze_dependency_modules(graph, &target_module_id)?;
    }

    // Generate test execution order (three-tier: dependencies → target → affected)
    let test_order = generate_test_order(&affected_modules, &dependency_modules, module_name);

    // Calculate summary statistics
    let total_affected = affected_modules.len();
    let total_dependencies = dependency_modules.len();
    let high_risk_count = affected_modules
        .iter()
        .chain(dependency_modules.iter())
        .filter(|m| matches!(m.risk_level, RiskLevel::High | RiskLevel::Critical))
        .count();

    Ok(ChangesetResult {
        target_module: module_name.to_string(),
        scope,
        affected_modules,
        dependency_modules,
        test_order,
        total_affected,
        total_dependencies,
        high_risk_count,
    })
}

/// Find a module by name in the graph
fn find_module_by_name(graph: &DependencyGraph, module_name: &str) -> Result<ModuleIdentifier> {
    for module in graph.all_modules() {
        if module.canonical_path == module_name {
            return Ok(module.clone());
        }
    }
    Err(anyhow::anyhow!(
        "Module '{}' not found in dependency graph",
        module_name
    ))
}

/// Analyze modules that would be affected by changes to the target (import-only)
fn analyze_affected_modules(
    graph: &DependencyGraph,
    target_module: &ModuleIdentifier,
) -> Result<Vec<ChangesetModule>> {
    // Use import-only traversal to get modules that directly import the target
    let mut affected_module_names = get_import_dependents(graph, target_module)?;

    // Filter out test modules
    affected_module_names
        .retain(|module_path| !module_path.contains(".tests.") && !module_path.ends_with(".tests"));

    // Filter out external modules
    affected_module_names.retain(|module_path| !is_external_module(graph, module_path));

    // Get dependent counts for risk assessment
    let dependent_counts = calculate_dependent_counts(graph)?;

    let mut modules = Vec::new();

    for module_name in affected_module_names {
        let dependent_count = dependent_counts.get(&module_name).unwrap_or(&0);
        let risk_level = assess_risk_level(*dependent_count, 0);

        modules.push(ChangesetModule {
            module_name,
            risk_level,
            dependent_count: *dependent_count,
            dependency_depth: 0, // Not used for affected modules
            is_external: false,
        });
    }

    // Sort by risk level (highest first)
    modules.sort_by(|a, b| b.risk_level.cmp(&a.risk_level));

    Ok(modules)
}

/// Analyze modules that the target depends on (import-only)
fn analyze_dependency_modules(
    graph: &DependencyGraph,
    target_module: &ModuleIdentifier,
) -> Result<Vec<ChangesetModule>> {
    // Use import-only traversal to get modules that target directly imports
    let mut dependency_module_names = get_import_dependencies(graph, target_module)?;

    // Filter out test modules
    dependency_module_names
        .retain(|module_path| !module_path.contains(".tests.") && !module_path.ends_with(".tests"));

    // Filter out external modules
    dependency_module_names.retain(|module_path| !is_external_module(graph, module_path));

    // Get dependent counts for risk assessment
    let dependent_counts = calculate_dependent_counts(graph)?;

    let mut modules = Vec::new();

    for module_name in dependency_module_names {
        let dependent_count = dependent_counts.get(&module_name).unwrap_or(&0);
        let risk_level = assess_risk_level(*dependent_count, 0);

        modules.push(ChangesetModule {
            module_name,
            risk_level,
            dependent_count: *dependent_count,
            dependency_depth: 1, // All direct imports
            is_external: false,
        });
    }

    // Sort by risk level (highest first)
    modules.sort_by(|a, b| b.risk_level.cmp(&a.risk_level));

    Ok(modules)
}

/// Calculate dependent counts for all modules
fn calculate_dependent_counts(graph: &DependencyGraph) -> Result<HashMap<String, usize>> {
    let mut counts = HashMap::new();

    for module in graph.all_modules() {
        let dependents = graph.get_dependents(module)?;
        counts.insert(module.canonical_path.clone(), dependents.len());
    }

    Ok(counts)
}

/// Assess risk level based on dependent count and other factors
fn assess_risk_level(dependent_count: usize, _dependency_depth: usize) -> RiskLevel {
    match dependent_count {
        0..=2 => RiskLevel::Low,
        3..=10 => RiskLevel::Medium,
        11..=50 => RiskLevel::High,
        _ => RiskLevel::Critical,
    }
}

/// Check if a module is external
fn is_external_module(graph: &DependencyGraph, module_name: &str) -> bool {
    for module in graph.all_modules() {
        if module.canonical_path == module_name {
            return module.origin == ModuleOrigin::External;
        }
    }
    false
}

/// Get modules that directly import the target module (import-only, no containment)
fn get_import_dependents(
    graph: &DependencyGraph,
    target_module: &ModuleIdentifier,
) -> Result<Vec<String>> {
    let mut dependents = Vec::new();

    // Iterate through all modules and check if they import the target
    for module in graph.all_modules() {
        let dependencies = graph.get_dependencies_with_types(module)?;

        for (dep_module, dep_type) in dependencies {
            // Only follow Imports edges, ignore containment relationships
            if dep_type == crate::graph::DependencyType::Imports
                && dep_module == target_module.canonical_path
            {
                dependents.push(module.canonical_path.clone());
                break; // Found one import, no need to check more from this module
            }
        }
    }

    Ok(dependents)
}

/// Get modules that the target module directly imports (import-only, no containment)
fn get_import_dependencies(
    graph: &DependencyGraph,
    target_module: &ModuleIdentifier,
) -> Result<Vec<String>> {
    let dependencies = graph.get_dependencies_with_types(target_module)?;

    // Filter to only include Imports relationships, exclude containment
    let import_deps: Vec<String> = dependencies
        .into_iter()
        .filter_map(|(dep_module, dep_type)| {
            if dep_type == crate::graph::DependencyType::Imports {
                Some(dep_module)
            } else {
                None
            }
        })
        .collect();

    Ok(import_deps)
}

/// Generate suggested test execution order using three-tier approach
fn generate_test_order(
    affected_modules: &[ChangesetModule],
    dependency_modules: &[ChangesetModule],
    target_module: &str,
) -> Vec<String> {
    let mut test_order = Vec::new();

    // Tier 1: Dependencies (test foundations first)
    // Sort dependencies by risk level (high risk first for early failure detection)
    let mut deps = dependency_modules.to_vec();
    deps.sort_by(|a, b| {
        b.risk_level
            .cmp(&a.risk_level)
            .then(a.module_name.cmp(&b.module_name))
    });

    for dep in deps {
        test_order.push(dep.module_name);
    }

    // Tier 2: Target module itself
    test_order.push(target_module.to_string());

    // Tier 3: Affected modules (test impact last)
    // Sort affected by dependency relationship - closer dependents first
    let mut affected = affected_modules.to_vec();
    affected.sort_by(|a, b| {
        // First by dependency depth (closer to target first), then by risk
        a.dependency_depth
            .cmp(&b.dependency_depth)
            .then(b.risk_level.cmp(&a.risk_level))
            .then(a.module_name.cmp(&b.module_name))
    });

    for affected_mod in affected {
        test_order.push(affected_mod.module_name);
    }

    // Remove duplicates while preserving order
    let mut seen = HashSet::new();
    test_order.retain(|module| seen.insert(module.clone()));

    test_order
}

/// Formatters for changeset results
pub mod formatters {
    use super::{ChangesetResult, RiskLevel};

    /// Formats results as human-readable text
    pub fn format_text_grouped(result: &ChangesetResult) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&format!("CHANGESET ANALYSIS: {}\n", result.target_module));
        output.push_str(&format!(
            "Affected: {} | Dependencies: {} | High Risk: {}\n\n",
            result.total_affected, result.total_dependencies, result.high_risk_count
        ));

        // Affected modules section
        if !result.affected_modules.is_empty() {
            output.push_str("AFFECTED MODULES (what breaks if target changes):\n");
            output.push_str("─────────────────────────────────────────────────\n");
            output.push_str(&format_modules_by_risk(&result.affected_modules));
            output.push('\n');
        }

        // Dependencies section
        if !result.dependency_modules.is_empty() {
            output.push_str("DEPENDENCIES (what target needs):\n");
            output.push_str("─────────────────────────────────\n");
            output.push_str(&format_modules_by_risk(&result.dependency_modules));
            output.push('\n');
        }

        // Test execution order
        if !result.test_order.is_empty() {
            output.push_str("SUGGESTED TEST ORDER (dependencies → target → affected):\n");
            output.push_str("─────────────────────────────────────────────────────────\n");

            let dependencies_count = result.dependency_modules.len();
            let target_position = dependencies_count + 1;

            for (i, module) in result.test_order.iter().enumerate() {
                let tier_info = if i < dependencies_count {
                    " [DEPENDENCY]"
                } else if i + 1 == target_position {
                    " [TARGET]"
                } else {
                    " [AFFECTED]"
                };

                output.push_str(&format!("{}. {}{}\n", i + 1, module, tier_info));
            }
            output.push_str("\nRationale: Test dependencies first (foundations), then target, then affected modules\n");
            output.push('\n');
        }

        // Risk assessment summary
        output.push_str("RISK ASSESSMENT:\n");
        output.push_str("────────────────\n");
        if result.high_risk_count > 0 {
            output.push_str(&format!(
                "⚠️  {} high-risk modules identified\n",
                result.high_risk_count
            ));
            output.push_str("• Consider breaking changes into smaller increments\n");
            output.push_str("• Focus testing efforts on high-risk modules\n");
            output.push_str("• Review integration points carefully\n");
        } else {
            output.push_str("✅ Low risk change - isolated impact\n");
            output.push_str("• Standard testing should be sufficient\n");
        }

        output
    }

    /// Format modules grouped by risk level
    fn format_modules_by_risk(modules: &[super::ChangesetModule]) -> String {
        use std::collections::HashMap;

        // Group by risk level
        let mut by_risk: HashMap<RiskLevel, Vec<&super::ChangesetModule>> = HashMap::new();
        for module in modules {
            by_risk
                .entry(module.risk_level.clone())
                .or_default()
                .push(module);
        }

        let mut output = String::new();

        // Show in order: Critical, High, Medium, Low
        for risk_level in [
            RiskLevel::Critical,
            RiskLevel::High,
            RiskLevel::Medium,
            RiskLevel::Low,
        ] {
            if let Some(risk_modules) = by_risk.get(&risk_level) {
                let risk_icon = match risk_level {
                    RiskLevel::Critical => "🔴",
                    RiskLevel::High => "🟠",
                    RiskLevel::Medium => "🟡",
                    RiskLevel::Low => "🟢",
                };

                output.push_str(&format!(
                    "{} {:?} Risk ({} modules):\n",
                    risk_icon,
                    risk_level,
                    risk_modules.len()
                ));

                for module in risk_modules {
                    output.push_str(&format!(
                        "  • {} ({} dependents)\n",
                        module.module_name, module.dependent_count
                    ));
                }
                output.push('\n');
            }
        }

        output
    }
}
