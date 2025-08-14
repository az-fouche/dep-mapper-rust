pub mod imports;
pub mod graph;

use std::fs;
use std::path::Path;
use imports::{extract_imports, ImportInfo, ModuleIdentifier, ModuleOrigin};
use graph::DependencyGraph;

/// Process a Python file and return its imports and a graph with the module added.
/// 
/// # Arguments
/// * `file_path` - Path to the Python file to analyze
/// 
/// # Returns
/// A tuple of (imports, graph_with_module, module_identifier)
pub fn process_python_file<P: AsRef<Path>>(file_path: P) -> Result<(Vec<ImportInfo>, DependencyGraph, ModuleIdentifier), Box<dyn std::error::Error>> {
    let file_path = file_path.as_ref();
    
    // Read the Python file
    let python_code = fs::read_to_string(file_path)?;
    
    // Extract imports
    let imports = extract_imports(&python_code)?;
    
    // Create module identifier
    let module_name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
        
    let module_id = ModuleIdentifier {
        origin: ModuleOrigin::Internal,
        canonical_path: module_name,
    };
    
    // Create graph and add module
    let mut graph = DependencyGraph::new();
    graph.add_module(module_id.clone())?;
    
    Ok((imports, graph, module_id))
}