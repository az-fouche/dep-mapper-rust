use std::collections::HashMap;
use std::path::PathBuf;
use petgraph::{Graph, Directed};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use serde::{Serialize, Deserialize};
use crate::imports::ImportInfo;

/// Information about a Python module including its location and imports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// File system path to the module
    pub file_path: PathBuf,
    /// Fully qualified module name (e.g., "package.submodule.module")
    pub module_name: String,
    /// List of imports found in this module
    pub imports: Vec<ImportInfo>,
}

/// A directed graph representing dependencies between Python modules.
/// 
/// Each node represents a module, and each edge represents a dependency
/// (import statement) from one module to another.
#[derive(Debug)]
pub struct DependencyGraph {
    /// The underlying directed graph structure
    graph: Graph<ModuleInfo, ImportInfo, Directed>,
    /// Fast lookup from module name to graph node index
    module_index: HashMap<String, NodeIndex>,
}

impl DependencyGraph {
    /// Creates a new empty dependency graph.
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            module_index: HashMap::new(),
        }
    }

    /// Adds a module to the graph.
    /// 
    /// Returns the node index for the newly added module.
    /// If a module with the same name already exists, it will be replaced.
    pub fn add_module(&mut self, module_info: ModuleInfo) -> NodeIndex {
        let module_name = module_info.module_name.clone();
        let node_idx = self.graph.add_node(module_info);
        self.module_index.insert(module_name, node_idx);
        node_idx
    }

    /// Adds a dependency edge between two modules.
    /// 
    /// # Arguments
    /// * `from_module` - The name of the module that imports
    /// * `to_module` - The name of the module being imported
    /// * `import_info` - Details about the import statement
    /// 
    /// # Errors
    /// Returns an error if either module is not found in the graph.
    pub fn add_dependency(&mut self, from_module: &str, to_module: &str, import_info: ImportInfo) -> Result<(), String> {
        let from_idx = self.module_index.get(from_module)
            .ok_or_else(|| format!("Module '{}' not found", from_module))?;
        let to_idx = self.module_index.get(to_module)
            .ok_or_else(|| format!("Module '{}' not found", to_module))?;
        
        self.graph.add_edge(*from_idx, *to_idx, import_info);
        Ok(())
    }

    /// Retrieves module information by name.
    pub fn get_module(&self, module_name: &str) -> Option<&ModuleInfo> {
        self.module_index.get(module_name)
            .and_then(|&idx| self.graph.node_weight(idx))
    }

    /// Gets all modules that the specified module depends on.
    /// 
    /// Returns a vector of tuples containing the dependent module and import info.
    pub fn get_dependencies(&self, module_name: &str) -> Vec<(&ModuleInfo, &ImportInfo)> {
        if let Some(&node_idx) = self.module_index.get(module_name) {
            self.graph
                .edges(node_idx)
                .filter_map(|edge| {
                    self.graph.node_weight(edge.target())
                        .map(|target_module| (target_module, edge.weight()))
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Gets all modules that depend on the specified module.
    /// 
    /// Returns a vector of modules that import the specified module.
    pub fn get_dependents(&self, module_name: &str) -> Vec<&ModuleInfo> {
        if let Some(&node_idx) = self.module_index.get(module_name) {
            self.graph
                .edges_directed(node_idx, petgraph::Incoming)
                .filter_map(|edge| self.graph.node_weight(edge.source()))
                .collect()
        } else {
            Vec::new()
        }
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
    pub fn all_modules(&self) -> impl Iterator<Item = &ModuleInfo> {
        self.graph.node_weights()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}