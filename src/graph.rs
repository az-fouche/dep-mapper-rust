use crate::imports::{ModuleIdentifier, ModuleOrigin};
use anyhow::Result;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::{Directed, Graph};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

/// Represents the type of dependency relationship between modules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyType {
    /// X imports Y (import/from import statement)
    Imports,
    /// X is included in Y (e.g., function/class defined in module)
    IncludedIn,
    /// X contains Y (e.g., module contains function/class)
    Contains,
    /// X is the module
    Is,
}

/// A directed graph representing dependencies between Python modules.
///
/// Each node represents a module, and each edge represents a dependency
/// relationship (import, containment, etc.) from one module to another.
#[derive(Debug)]
pub struct DependencyGraph {
    /// The underlying directed graph structure where each node contains a module path string
    /// and each edge contains the type of dependency relationship
    graph: Graph<String, DependencyType, Directed>,
    /// Fast lookup from module identifier to graph node index
    module_index: HashMap<ModuleIdentifier, NodeIndex>,
}

impl DependencyGraph {
    /// Creates a new empty dependency graph.
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            module_index: HashMap::new(),
        }
    }

    /// Adds a module to the graph if not already known.
    ///
    /// Returns the node index for the newly added module.
    pub fn add_module(&mut self, module_id: ModuleIdentifier) -> NodeIndex {
        if let Some(&existing_idx) = self.module_index.get(&module_id) {
            existing_idx
        } else {
            let node_idx = self.graph.add_node(module_id.canonical_path.clone());
            self.module_index.insert(module_id, node_idx);
            node_idx
        }
    }

    /// Adds a dependency edge between two modules.
    ///
    /// # Arguments
    /// * `from_module` - The identifier of the source module
    /// * `to_module` - The identifier of the target module
    /// * `dependency_type` - The type of relationship between the modules
    ///
    /// # Errors
    /// Returns an error if either module is not found in the graph.
    pub fn add_dependency(
        &mut self,
        from_module: &ModuleIdentifier,
        to_module: &ModuleIdentifier,
        dependency_type: DependencyType,
    ) -> Result<()> {
        let from_idx = self
            .module_index
            .get(from_module)
            .ok_or_else(|| anyhow::anyhow!("Module '{}' not found", from_module.canonical_path))?;
        let to_idx = self
            .module_index
            .get(to_module)
            .ok_or_else(|| anyhow::anyhow!("Module '{}' not found", to_module.canonical_path))?;

        self.graph.add_edge(*from_idx, *to_idx, dependency_type);
        Ok(())
    }

    /// Gets all modules that the specified module depends on.
    ///
    /// Returns a vector of module identifiers that this module imports.
    ///
    /// # Errors
    /// Returns an error if the module is not found in the graph.
    pub fn get_dependencies(&self, module_id: &ModuleIdentifier) -> Result<Vec<String>> {
        let node_idx = self.get_node_index(module_id)?;

        Ok(self
            .graph
            .edges(node_idx)
            .filter_map(|edge| self.graph.node_weight(edge.target()))
            .cloned()
            .collect())
    }

    /// Returns NodeIndex of a module_id or an error if not found.
    fn get_node_index(&self, module_id: &ModuleIdentifier) -> Result<NodeIndex> {
        self.module_index.get(module_id).copied().ok_or_else(|| {
            anyhow::anyhow!("Module '{}' not found in graph", module_id.canonical_path)
        })
    }

    /// Collect all descendant nodes reachable by following `Contains` edges.
    ///
    /// Includes the starting node if `include_self` is true.
    fn descendants_via_contains(
        &self,
        module_id: &ModuleIdentifier,
        include_self: bool,
    ) -> Result<Vec<NodeIndex>> {
        let start = self.get_node_index(module_id)?;
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        if include_self {
            visited.insert(start);
            result.push(start);
        }
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            for edge in self.graph.edges(current) {
                if matches!(edge.weight(), DependencyType::Contains) {
                    let child = edge.target();
                    if visited.insert(child) {
                        result.push(child);
                        queue.push_back(child);
                    }
                }
            }
        }

        Ok(result)
    }

    /// Gets all modules that depend on the specified module.
    ///
    /// Returns a vector of module identifiers that import the specified module.
    ///
    /// # Errors
    /// Returns an error if the module is not found in the graph.
    pub fn get_dependents(&self, module_id: &ModuleIdentifier) -> Result<Vec<String>> {
        let node_idx = self.get_node_index(module_id)?;

        Ok(self
            .graph
            .edges_directed(node_idx, petgraph::Incoming)
            .filter_map(|edge| self.graph.node_weight(edge.source()))
            .cloned()
            .collect())
    }

    /// Gets all modules that the specified module depends on with their dependency types.
    ///
    /// Returns a vector of tuples containing (target_module, dependency_type).
    ///
    /// # Errors
    /// Returns an error if the module is not found in the graph.
    pub fn get_dependencies_with_types(
        &self,
        module_id: &ModuleIdentifier,
    ) -> Result<Vec<(String, DependencyType)>> {
        let node_idx = self.get_node_index(module_id)?;

        Ok(self
            .graph
            .edges(node_idx)
            .filter_map(|edge| {
                self.graph
                    .node_weight(edge.target())
                    .map(|module| (module.clone(), edge.weight().clone()))
            })
            .collect())
    }

    /// Gets all modules that depend on the specified module **or any of its descendants**.
    ///
    /// Traverses `Contains` edges downward, then collects incoming edges to each visited node.
    /// Returns (dependent_module, dependency_type_on_that_child). De-duplicates by dependent module name.
    pub fn get_transitive_dependents_with_types(
        &self,
        module_id: &ModuleIdentifier,
    ) -> Result<Vec<(String, DependencyType)>> {
        let descendant_nodes = self.descendants_via_contains(module_id, true)?;
        let mut seen_dependents = HashSet::new();
        let mut result = Vec::new();

        result.push((module_id.canonical_path.clone(), DependencyType::Is));

        for node in descendant_nodes {
            for edge in self.graph.edges_directed(node, petgraph::Incoming) {
                if *edge.weight() == DependencyType::Contains {
                    continue;
                }
                if let Some(dependent_module) = self.graph.node_weight(edge.source()) {
                    if seen_dependents.insert(dependent_module.clone()) {
                        result.push((dependent_module.clone(), edge.weight().clone()));
                    }
                }
            }
        }

        Ok(result)
    }

    /// Gets all modules that the specified module **or any of its descendants** depend on.
    ///
    /// Traverses `Contains` edges downward, then collects outgoing edges from each visited node.
    /// Returns (dependency_module, dependency_type_from_that_child). De-duplicates by dependency module name.
    pub fn get_transitive_dependencies_with_types(
        &self,
        module_id: &ModuleIdentifier,
    ) -> Result<Vec<(String, DependencyType)>> {
        let descendant_nodes = self.descendants_via_contains(module_id, true)?;
        let mut seen_dependencies = HashSet::new();
        let mut result = Vec::new();

        for node in descendant_nodes {
            for edge in self.graph.edges(node) {
                if *edge.weight() == DependencyType::Contains {
                    continue;
                }
                if let Some(dependency_module) = self.graph.node_weight(edge.target()) {
                    if seen_dependencies.insert(dependency_module.clone()) {
                        result.push((dependency_module.clone(), edge.weight().clone()));
                    }
                }
            }
        }

        Ok(result)
    }

    /// Returns the total number of modules in the graph.
    pub fn module_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Returns the total number of dependency relationships in the graph.
    pub fn dependency_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Returns an iterator over all modules in the graph.
    pub fn all_modules(&self) -> impl Iterator<Item = &ModuleIdentifier> {
        self.module_index.keys()
    }
}

/// Utility functions for working with dependency graphs
pub mod utils {
    use super::*;

    /// Adds Contains/IncludedIn relationships based on module path hierarchy.
    ///
    /// For each module with dots in its path, creates bidirectional relationships
    /// with its direct parent module.
    pub fn add_containment_relationships(graph: &mut DependencyGraph) -> Result<()> {
        let modules: Vec<ModuleIdentifier> = graph.all_modules().cloned().collect();

        for module in &modules {
            if let Some(parent_path) = get_direct_parent_module(&module.canonical_path) {
                let parent_module = ModuleIdentifier {
                    origin: module.origin.clone(),
                    canonical_path: parent_path,
                };

                graph.add_module(parent_module.clone());
                graph.add_dependency(&parent_module, &module, DependencyType::Contains)?;
                graph.add_dependency(&module, &parent_module, DependencyType::IncludedIn)?;
            }
        }

        Ok(())
    }

    /// Extracts the direct parent module from a module path.
    ///
    /// Returns the immediate parent module path, or None if the module is top-level.
    pub fn get_direct_parent_module(module_path: &str) -> Option<String> {
        if let Some(last_dot) = module_path.rfind('.') {
            Some(module_path[..last_dot].to_string())
        } else {
            None
        }
    }
}

impl fmt::Display for DependencyGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_dependency_graph(self, f)
    }
}

/// Formats a dependency graph for display
fn format_dependency_graph(graph: &DependencyGraph, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    writeln!(f, "--- Dependency Graph ---")?;
    writeln!(
        f,
        "Modules: {}, Dependencies: {}\n",
        graph.module_count(),
        graph.dependency_count()
    )?;

    let mut internal_modules: Vec<_> = graph
        .all_modules()
        .filter(|m| m.origin == ModuleOrigin::Internal)
        .collect();
    internal_modules.sort_by(|a, b| a.canonical_path.cmp(&b.canonical_path));

    for module in internal_modules {
        let dependencies = graph
            .get_dependencies_with_types(module)
            .unwrap_or_default();

        if dependencies.is_empty() {
            writeln!(f, "{} -> (no dependencies)", module.canonical_path)?;
        } else {
            writeln!(
                f,
                "{} -> ({} deps)",
                module.canonical_path,
                dependencies.len()
            )?;
            for (dep_module, dep_type) in dependencies {
                writeln!(f, "  -> {} ({:?})", dep_module, dep_type)?;
            }
        }
    }

    Ok(())
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imports::{ModuleIdentifier, ModuleOrigin};

    fn create_test_module_id(name: &str, origin: ModuleOrigin) -> ModuleIdentifier {
        ModuleIdentifier {
            origin,
            canonical_path: name.to_string(),
        }
    }

    #[test]
    fn test_new_graph() {
        let graph = DependencyGraph::new();
        assert_eq!(graph.module_count(), 0);
        assert_eq!(graph.dependency_count(), 0);
    }

    #[test]
    fn test_add_module() {
        let mut graph = DependencyGraph::new();
        let module_id = create_test_module_id("test.module", ModuleOrigin::Internal);

        graph.add_module(module_id.clone());

        assert_eq!(graph.module_count(), 1);
        // Module should exist in the graph
        let all_modules: Vec<_> = graph.all_modules().collect();
        assert!(
            all_modules
                .iter()
                .any(|m| m.canonical_path == "test.module")
        );
    }

    #[test]
    fn test_module_counts() {
        let mut graph = DependencyGraph::new();

        graph.add_module(create_test_module_id("module1", ModuleOrigin::Internal));
        graph.add_module(create_test_module_id("module2", ModuleOrigin::Internal));
        graph.add_module(create_test_module_id("module3", ModuleOrigin::Internal));

        assert_eq!(graph.module_count(), 3);
        assert_eq!(graph.dependency_count(), 0);
    }

    #[test]
    fn test_add_dependency() {
        let mut graph = DependencyGraph::new();

        let module1 = create_test_module_id("module1", ModuleOrigin::Internal);
        let module2 = create_test_module_id("module2", ModuleOrigin::Internal);

        graph.add_module(module1.clone());
        graph.add_module(module2.clone());

        let result = graph.add_dependency(&module1, &module2, DependencyType::Imports);

        assert!(result.is_ok());
        assert_eq!(graph.dependency_count(), 1);
    }

    #[test]
    fn test_get_dependencies() {
        let mut graph = DependencyGraph::new();

        let main_id = create_test_module_id("main", ModuleOrigin::Internal);
        let utils_id = create_test_module_id("utils", ModuleOrigin::Internal);
        let config_id = create_test_module_id("config", ModuleOrigin::Internal);

        graph.add_module(main_id.clone());
        graph.add_module(utils_id.clone());
        graph.add_module(config_id.clone());

        graph
            .add_dependency(&main_id, &utils_id, DependencyType::Imports)
            .unwrap();
        graph
            .add_dependency(&main_id, &config_id, DependencyType::Imports)
            .unwrap();

        let deps = graph.get_dependencies(&main_id).unwrap();
        assert_eq!(deps.len(), 2);

        assert!(deps.contains(&"utils".to_string()));
        assert!(deps.contains(&"config".to_string()));
    }

    #[test]
    fn test_get_dependents() {
        let mut graph = DependencyGraph::new();

        let utils_id = create_test_module_id("utils", ModuleOrigin::Internal);
        let main_id = create_test_module_id("main", ModuleOrigin::Internal);
        let tests_id = create_test_module_id("tests", ModuleOrigin::Internal);

        graph.add_module(utils_id.clone());
        graph.add_module(main_id.clone());
        graph.add_module(tests_id.clone());

        graph
            .add_dependency(&main_id, &utils_id, DependencyType::Imports)
            .unwrap();
        graph
            .add_dependency(&tests_id, &utils_id, DependencyType::Imports)
            .unwrap();

        let dependents = graph.get_dependents(&utils_id).unwrap();
        assert_eq!(dependents.len(), 2);

        assert!(dependents.contains(&"main".to_string()));
        assert!(dependents.contains(&"tests".to_string()));
    }

    #[test]
    fn test_add_dependency_missing_modules() {
        let mut graph = DependencyGraph::new();

        let existing_id = create_test_module_id("existing", ModuleOrigin::Internal);
        let missing_id = create_test_module_id("missing", ModuleOrigin::Internal);

        graph.add_module(existing_id.clone());

        let result = graph.add_dependency(&existing_id, &missing_id, DependencyType::Imports);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Module 'missing' not found")
        );

        let result2 = graph.add_dependency(&missing_id, &existing_id, DependencyType::Imports);
        assert!(result2.is_err());
        assert!(
            result2
                .unwrap_err()
                .to_string()
                .contains("Module 'missing' not found")
        );
    }

    #[test]
    fn test_dependencies_of_nonexistent_module() {
        let graph = DependencyGraph::new();
        let nonexistent_id = create_test_module_id("nonexistent", ModuleOrigin::Internal);
        let result = graph.get_dependencies(&nonexistent_id);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Module 'nonexistent' not found")
        );
    }

    #[test]
    fn test_dependents_of_nonexistent_module() {
        let graph = DependencyGraph::new();
        let nonexistent_id = create_test_module_id("nonexistent", ModuleOrigin::Internal);
        let result = graph.get_dependents(&nonexistent_id);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Module 'nonexistent' not found")
        );
    }

    #[test]
    fn test_all_modules_iterator() {
        let mut graph = DependencyGraph::new();

        graph.add_module(create_test_module_id("module1", ModuleOrigin::Internal));
        graph.add_module(create_test_module_id("module2", ModuleOrigin::Internal));
        graph.add_module(create_test_module_id("module3", ModuleOrigin::Internal));

        let all_modules: Vec<&ModuleIdentifier> = graph.all_modules().collect();
        assert_eq!(all_modules.len(), 3);

        let module_names: Vec<&str> = all_modules
            .iter()
            .map(|m| m.canonical_path.as_str())
            .collect();
        assert!(module_names.contains(&"module1"));
        assert!(module_names.contains(&"module2"));
        assert!(module_names.contains(&"module3"));
    }

    #[test]
    fn test_module_replacement() {
        let mut graph = DependencyGraph::new();

        let module_id = create_test_module_id("module1", ModuleOrigin::Internal);
        graph.add_module(module_id.clone());
        assert_eq!(graph.module_count(), 1);

        // Adding same module again - count should remain 1
        graph.add_module(module_id.clone());
        assert_eq!(graph.module_count(), 1);
    }

    #[test]
    fn test_get_direct_parent_module() {
        use super::utils::get_direct_parent_module;
        assert_eq!(
            get_direct_parent_module("numpy.testing.utils"),
            Some("numpy.testing".to_string())
        );
        assert_eq!(
            get_direct_parent_module("numpy.testing"),
            Some("numpy".to_string())
        );
        assert_eq!(get_direct_parent_module("numpy"), None);
        assert_eq!(get_direct_parent_module(""), None);
        assert_eq!(get_direct_parent_module("single"), None);
    }

    #[test]
    fn test_add_containment_relationships() {
        use super::utils::add_containment_relationships;
        let mut graph = DependencyGraph::new();

        // Add modules with hierarchical names
        let numpy_id = create_test_module_id("numpy", ModuleOrigin::External);
        let numpy_testing_id = create_test_module_id("numpy.testing", ModuleOrigin::External);
        let numpy_testing_utils_id =
            create_test_module_id("numpy.testing.utils", ModuleOrigin::External);
        let scipy_id = create_test_module_id("scipy", ModuleOrigin::External);

        graph.add_module(numpy_id.clone());
        graph.add_module(numpy_testing_id.clone());
        graph.add_module(numpy_testing_utils_id.clone());
        graph.add_module(scipy_id.clone());

        // Initially should have 4 modules, 0 dependencies
        assert_eq!(graph.module_count(), 4);
        assert_eq!(graph.dependency_count(), 0);

        // Add containment relationships
        add_containment_relationships(&mut graph).unwrap();

        // Should have same modules but new dependencies
        assert_eq!(graph.module_count(), 4);
        assert_eq!(graph.dependency_count(), 4); // 2 bidirectional relationships

        // Test specific relationships
        let numpy_testing_deps = graph
            .get_dependencies_with_types(&numpy_testing_id)
            .unwrap();
        assert_eq!(numpy_testing_deps.len(), 2); // Both IncludedIn numpy and Contains numpy.testing.utils
        assert!(numpy_testing_deps.contains(&("numpy".to_string(), DependencyType::IncludedIn)));
        assert!(
            numpy_testing_deps
                .contains(&("numpy.testing.utils".to_string(), DependencyType::Contains))
        );

        let numpy_deps = graph.get_dependencies_with_types(&numpy_id).unwrap();
        assert_eq!(numpy_deps.len(), 1);
        assert!(numpy_deps.contains(&("numpy.testing".to_string(), DependencyType::Contains)));

        let numpy_testing_utils_deps = graph
            .get_dependencies_with_types(&numpy_testing_utils_id)
            .unwrap();
        assert_eq!(numpy_testing_utils_deps.len(), 1);
        assert!(
            numpy_testing_utils_deps
                .contains(&("numpy.testing".to_string(), DependencyType::IncludedIn))
        );

        // scipy should have no dependencies (top-level module)
        let scipy_deps = graph.get_dependencies_with_types(&scipy_id).unwrap();
        assert_eq!(scipy_deps.len(), 0);
    }

    #[test]
    fn test_get_dependencies_with_types() {
        let mut graph = DependencyGraph::new();

        let module1 = create_test_module_id("module1", ModuleOrigin::Internal);
        let module2 = create_test_module_id("module2", ModuleOrigin::Internal);
        let module3 = create_test_module_id("module3", ModuleOrigin::Internal);

        graph.add_module(module1.clone());
        graph.add_module(module2.clone());
        graph.add_module(module3.clone());

        // Add different types of dependencies
        graph
            .add_dependency(&module1, &module2, DependencyType::Imports)
            .unwrap();
        graph
            .add_dependency(&module1, &module3, DependencyType::Contains)
            .unwrap();

        let deps = graph.get_dependencies_with_types(&module1).unwrap();
        assert_eq!(deps.len(), 2);

        assert!(deps.contains(&("module2".to_string(), DependencyType::Imports)));
        assert!(deps.contains(&("module3".to_string(), DependencyType::Contains)));
    }
}
