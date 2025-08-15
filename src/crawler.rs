use std::fs;
use std::path::Path;
use crate::imports::{extract_module_dependencies_with_context, ModuleIdentifier, ModuleOrigin};
use crate::graph::DependencyGraph;

/// Builds a dependency graph from all Python files in a directory (non-recursive).
pub fn build_directory_dependency_graph(dir_path: &Path) -> Result<DependencyGraph, Box<dyn std::error::Error>> {
    let python_files = analyze_python_directory(dir_path)?;
    let mut graph = DependencyGraph::new();
    
    for file_path in &python_files {
        match analyze_python_file(file_path) {
            Ok((module_id, dependencies, _)) => {
                graph.add_module(module_id.clone())?;
                for dep in &dependencies {
                    graph.add_module(dep.clone()).ok(); // Ignore duplicates
                    graph.add_dependency(&module_id, dep)?;
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to analyze '{}': {}", file_path.display(), e);
                continue;
            }
        }
    }
    
    Ok(graph)
}

/// Discovers all Python files in a directory (non-recursive).
pub fn analyze_python_directory(dir_path: &Path) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    if !dir_path.is_dir() {
        return Err(format!("Path '{}' is not a directory", dir_path.display()).into());
    }
    
    let mut python_files = Vec::new();
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "py" {
                    python_files.push(path);
                }
            }
        }
    }
    
    // Sort files for consistent output
    python_files.sort();
    
    Ok(python_files)
}

/// Analyzes a single Python file and returns its module info, dependencies, and a single-file graph.
pub fn analyze_python_file(file_path: &Path) -> Result<(ModuleIdentifier, Vec<ModuleIdentifier>, DependencyGraph), Box<dyn std::error::Error>> {
    let python_code = fs::read_to_string(file_path)?;
    let current_dir = std::env::current_dir()?;
    let dependencies = extract_module_dependencies_with_context(&python_code, file_path, &current_dir)?;
    
    // Create module identifier for this file
    let module_name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    let module_id = ModuleIdentifier {
        origin: ModuleOrigin::Internal,
        canonical_path: module_name,
    };
    
    // Create graph with module and dependencies
    let mut graph = DependencyGraph::new();
    graph.add_module(module_id.clone())?;
    for dep in &dependencies {
        graph.add_module(dep.clone()).ok(); // Ignore duplicates
        graph.add_dependency(&module_id, dep)?;
    }
    
    Ok((module_id, dependencies, graph))
}