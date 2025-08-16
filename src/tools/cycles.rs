use crate::graph::{DependencyGraph, DependencyType};
use crate::imports::ModuleIdentifier;
use anyhow::{Context, Result, anyhow};
use petgraph::graph::NodeIndex;
use std::collections::{HashMap, HashSet};

/// Represents a detected circular dependency cycle
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Cycle {
    /// The modules in the cycle, in order (without repeating the first at the end)
    pub modules: Vec<String>,
}

impl Cycle {
    pub fn new(modules: Vec<String>) -> Self {
        Self { modules }
    }

    /// "a → b → c → a"
    pub fn format_cycle(&self) -> String {
        if self.modules.is_empty() {
            return String::new();
        }
        let mut s = self.modules.join(" → ");
        s.push_str(" → ");
        s.push_str(&self.modules[0]);
        s
    }
}

#[derive(Debug)]
pub struct CycleResult {
    pub cycles: Vec<Cycle>,
}

impl CycleResult {
    pub fn new(cycles: Vec<Cycle>) -> Self {
        Self { cycles }
    }

    pub fn cycle_count(&self) -> usize {
        self.cycles.len()
    }
}

/// Detect circular import dependencies using transitive dependency propagation.
/// If a.x imports b.y, this creates a module-level dependency a -> b.
pub fn detect_cycles(graph: &DependencyGraph) -> Result<CycleResult> {
    // 1) Build node <-> module maps once.
    let mut module_to_node: HashMap<String, NodeIndex> = HashMap::new();
    let mut node_to_module: HashMap<NodeIndex, String> = HashMap::new();

    for module in graph.all_modules() {
        let idx = graph
            .get_node_index(module)
            .with_context(|| format!("Missing node index for {}", module.canonical_path))?;
        module_to_node.insert(module.canonical_path.clone(), idx);
        node_to_module.insert(idx, module.canonical_path.clone());
    }

    // 2) Build adjacency with transitive dependencies (propagated through submodules).
    let mut adj: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
    for (module_name, &src) in module_to_node.iter() {
        let module_id = find_module_by_name_cached(graph, module_name)
            .with_context(|| format!("Module '{}' not found", module_name))?;
        let deps = graph
            .get_transitive_dependencies_with_types(&module_id)
            .with_context(|| format!("Failed to get transitive deps for '{}'", module_name))?;

        let import_targets = deps
            .into_iter()
            .filter(|(_, ty)| *ty == DependencyType::Imports)
            .filter_map(|(dep_name, _)| module_to_node.get(&dep_name).copied())
            .collect::<Vec<_>>();

        adj.entry(src).or_default().extend(import_targets);
    }

    // 3) DFS with explicit recursion stack to find back-edges -> cycles.
    let mut visited: HashSet<NodeIndex> = HashSet::new();
    let mut stack: Vec<NodeIndex> = Vec::new();
    let mut on_stack: HashSet<NodeIndex> = HashSet::new();

    // Use a set of canonicalized cycle signatures to deduplicate.
    let mut seen: HashSet<Vec<String>> = HashSet::new();
    let mut out: Vec<Cycle> = Vec::new();

    for &start in node_to_module.keys() {
        if !visited.contains(&start) {
            dfs_cycles(
                start,
                &adj,
                &mut visited,
                &mut stack,
                &mut on_stack,
                &node_to_module,
                &mut seen,
                &mut out,
            )?;
        }
    }

    Ok(CycleResult::new(out))
}

fn dfs_cycles(
    node: NodeIndex,
    adj: &HashMap<NodeIndex, Vec<NodeIndex>>,
    visited: &mut HashSet<NodeIndex>,
    stack: &mut Vec<NodeIndex>,
    on_stack: &mut HashSet<NodeIndex>,
    node_to_module: &HashMap<NodeIndex, String>,
    seen: &mut HashSet<Vec<String>>,
    out: &mut Vec<Cycle>,
) -> Result<()> {
    visited.insert(node);
    stack.push(node);
    on_stack.insert(node);

    if let Some(neighs) = adj.get(&node) {
        for &v in neighs {
            if !visited.contains(&v) {
                dfs_cycles(v, adj, visited, stack, on_stack, node_to_module, seen, out)?;
            } else if on_stack.contains(&v) {
                // Found a back-edge; extract cycle from v .. current node.
                if let Some(pos) = stack.iter().position(|&n| n == v) {
                    let cycle_slice = &stack[pos..];
                    let mut names: Vec<String> = cycle_slice
                        .iter()
                        .map(|n| node_to_module.get(n).cloned().unwrap_or_default())
                        .collect();

                    // Normalize to avoid duplicates (rotation & direction).
                    normalize_cycle(&mut names);

                    if seen.insert(names.clone()) && !names.is_empty() {
                        out.push(Cycle::new(names));
                    }
                }
            }
        }
    }

    on_stack.remove(&node);
    stack.pop();
    Ok(())
}

/// Normalize a cycle to a canonical representation:
/// - rotate so the lexicographically smallest string is first
/// - pick the lexicographically smaller between forward and reversed cycles
fn normalize_cycle(names: &mut Vec<String>) {
    if names.is_empty() {
        return;
    }

    // Rotation to smallest first
    let (min_i, _) = names
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.cmp(b))
        .unwrap();
    names.rotate_left(min_i);

    // Compare with reversed-rotated form to ensure unique direction
    let mut rev = names.clone();
    rev.reverse();
    let (min_i_rev, _) = rev
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.cmp(b))
        .unwrap();
    rev.rotate_left(min_i_rev);

    if rev < *names {
        *names = rev;
    }
}

/// Fast path using the already-built maps; avoids scanning all modules repeatedly.
fn find_module_by_name_cached(
    graph: &DependencyGraph,
    module_name: &str,
) -> Result<ModuleIdentifier> {
    graph
        .all_modules()
        .find(|m| m.canonical_path == module_name)
        .cloned()
        .ok_or_else(|| anyhow!("Module '{}' not found", module_name))
}

pub mod formatters {
    use super::CycleResult;

    pub fn format_text_grouped(result: &CycleResult) -> String {
        let mut output = String::new();
        if result.cycles.is_empty() {
            output.push_str("No circular dependencies found.\n");
            return output;
        }

        output.push_str("Circular dependencies found:\n");
        for (i, cycle) in result.cycles.iter().enumerate() {
            output.push_str(&format!("Cycle {}: {}\n", i + 1, cycle.format_cycle()));
        }
        output.push_str(&format!(
            "Total: {} cycle{}\n",
            result.cycle_count(),
            if result.cycle_count() == 1 { "" } else { "s" }
        ));
        output
    }
}
