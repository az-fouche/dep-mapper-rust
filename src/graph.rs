use crate::imports::{ModuleIdentifier, ModuleOrigin};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::{Directed, Graph};
use std::collections::HashMap;
use std::fmt;

/// A directed graph representing dependencies between Python modules.
///
/// Each node represents a module, and each edge represents a dependency
/// (import statement) from one module to another.
#[derive(Debug)]
pub struct DependencyGraph {
    /// The underlying directed graph structure
    graph: Graph<ModuleIdentifier, (), Directed>,
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
        if !self.module_index.contains_key(&module_id) {
            let node_idx = self.graph.add_node(module_id.clone());
            self.module_index.insert(module_id.clone(), node_idx);
        }
        *self.module_index.get(&module_id.clone()).unwrap()
    }

    /// Adds a dependency edge between two modules.
    ///
    /// # Arguments
    /// * `from_module` - The identifier of the module that imports
    /// * `to_module` - The identifier of the module being imported
    ///
    /// # Errors
    /// Returns an error if either module is not found in the graph.
    pub fn add_dependency(
        &mut self,
        from_module: &ModuleIdentifier,
        to_module: &ModuleIdentifier,
    ) -> Result<(), String> {
        let from_idx = self
            .module_index
            .get(from_module)
            .ok_or_else(|| format!("Module '{}' not found", from_module.canonical_path))?;
        let to_idx = self
            .module_index
            .get(to_module)
            .ok_or_else(|| format!("Module '{}' not found", to_module.canonical_path))?;

        self.graph.add_edge(*from_idx, *to_idx, ());
        Ok(())
    }

    /// Retrieves module identifier.
    pub fn get_module(&self, module_id: &ModuleIdentifier) -> Option<&ModuleIdentifier> {
        self.module_index
            .get(module_id)
            .and_then(|&idx| self.graph.node_weight(idx))
    }

    /// Gets all modules that the specified module depends on.
    ///
    /// Returns a vector of module identifiers that this module imports.
    ///
    /// # Errors
    /// Returns an error if the module is not found in the graph.
    pub fn get_dependencies(
        &self,
        module_id: &ModuleIdentifier,
    ) -> Result<Vec<&ModuleIdentifier>, String> {
        let node_idx = self
            .module_index
            .get(module_id)
            .ok_or_else(|| format!("Module '{}' not found in graph", module_id.canonical_path))?;

        Ok(self
            .graph
            .edges(*node_idx)
            .filter_map(|edge| self.graph.node_weight(edge.target()))
            .collect())
    }

    /// Gets all modules that depend on the specified module.
    ///
    /// Returns a vector of module identifiers that import the specified module.
    ///
    /// # Errors
    /// Returns an error if the module is not found in the graph.
    pub fn get_dependents(
        &self,
        module_id: &ModuleIdentifier,
    ) -> Result<Vec<&ModuleIdentifier>, String> {
        let node_idx = self
            .module_index
            .get(module_id)
            .ok_or_else(|| format!("Module '{}' not found in graph", module_id.canonical_path))?;

        Ok(self
            .graph
            .edges_directed(*node_idx, petgraph::Incoming)
            .filter_map(|edge| self.graph.node_weight(edge.source()))
            .collect())
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
        self.graph.node_weights()
    }
}

impl fmt::Display for DependencyGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "--- Dependency Graph ---")?;
        writeln!(
            f,
            "Modules: {}, Dependencies: {}\n",
            self.module_count(),
            self.dependency_count()
        )?;

        let mut modules: Vec<_> = self.all_modules().collect();
        modules.sort_by(|a, b| a.canonical_path.cmp(&b.canonical_path));

        for module in modules {
            if module.origin != ModuleOrigin::Internal {
                continue;
            }
            let dependencies = self.get_dependencies(module).unwrap_or_default();
            writeln!(f, "{} -> ({} deps)", module.canonical_path, dependencies.len())?;
        }

        Ok(())
    }
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
        assert!(graph.get_module(&module_id).is_some());
        assert_eq!(
            graph.get_module(&module_id).unwrap().canonical_path,
            "test.module"
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

        let result = graph.add_dependency(&module1, &module2);

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

        graph.add_dependency(&main_id, &utils_id).unwrap();
        graph.add_dependency(&main_id, &config_id).unwrap();

        let deps = graph.get_dependencies(&main_id).unwrap();
        assert_eq!(deps.len(), 2);

        let dep_names: Vec<&str> = deps
            .iter()
            .map(|module| module.canonical_path.as_str())
            .collect();
        assert!(dep_names.contains(&"utils"));
        assert!(dep_names.contains(&"config"));
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

        graph.add_dependency(&main_id, &utils_id).unwrap();
        graph.add_dependency(&tests_id, &utils_id).unwrap();

        let dependents = graph.get_dependents(&utils_id).unwrap();
        assert_eq!(dependents.len(), 2);

        let dependent_names: Vec<&str> = dependents
            .iter()
            .map(|module| module.canonical_path.as_str())
            .collect();
        assert!(dependent_names.contains(&"main"));
        assert!(dependent_names.contains(&"tests"));
    }

    #[test]
    fn test_add_dependency_missing_modules() {
        let mut graph = DependencyGraph::new();

        let existing_id = create_test_module_id("existing", ModuleOrigin::Internal);
        let missing_id = create_test_module_id("missing", ModuleOrigin::Internal);

        graph.add_module(existing_id.clone());

        let result = graph.add_dependency(&existing_id, &missing_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Module 'missing' not found"));

        let result2 = graph.add_dependency(&missing_id, &existing_id);
        assert!(result2.is_err());
        assert!(result2.unwrap_err().contains("Module 'missing' not found"));
    }

    #[test]
    fn test_get_nonexistent_module() {
        let graph = DependencyGraph::new();
        let nonexistent_id = create_test_module_id("nonexistent", ModuleOrigin::Internal);
        assert!(graph.get_module(&nonexistent_id).is_none());
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
        assert!(graph.get_module(&module_id).is_some());

        // Count should remain 1
        graph.add_module(module_id.clone());
        assert_eq!(graph.module_count(), 1);
    }
}
