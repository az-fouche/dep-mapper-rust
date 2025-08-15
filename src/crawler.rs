use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::imports::{extract_module_dependencies_with_context, ModuleIdentifier, ModuleOrigin};
use crate::graph::DependencyGraph;

/// Builds a dependency graph from all Python files in a directory (recursive).
pub fn build_directory_dependency_graph(dir_path: &Path, max_files: Option<usize>) -> Result<DependencyGraph, Box<dyn std::error::Error>> {
    let python_files = analyze_python_directory_recursive(dir_path, max_files)?;
    let mut graph = DependencyGraph::new();
    
    for file_path in &python_files {
        match analyze_python_file_with_package(file_path, dir_path) {
            Ok((module_id, dependencies)) => {
                graph.add_module(module_id.clone()).ok(); // Ignore duplicates - module might be added as dependency first
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

/// Discovers all Python files in a directory and its subdirectories (recursive).
pub fn analyze_python_directory_recursive(dir_path: &Path, max_files: Option<usize>) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    if !dir_path.is_dir() {
        return Err(format!("Path '{}' is not a directory", dir_path.display()).into());
    }
    
    let mut python_files = Vec::new();
    
    for entry in WalkDir::new(dir_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            // Skip directories starting with dot or named 'tests'
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    if name.starts_with('.') || name == "tests" {
                        return false;
                    }
                }
            }
            e.file_type().is_file()
        })
    {
        let path = entry.path();
        if let Some(extension) = path.extension() {
            if extension == "py" {
                python_files.push(path.to_path_buf());
            }
        }
    }
    
    // Sort files for consistent output
    python_files.sort();
    
    // Limit files if max_files is specified
    if let Some(max) = max_files {
        python_files.truncate(max);
    }
    
    Ok(python_files)
}

/// Analyzes a single Python file and returns the module identifier and its dependencies.
pub fn analyze_python_file(file_path: &Path) -> Result<(ModuleIdentifier, Vec<ModuleIdentifier>), Box<dyn std::error::Error>> {
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
    
    Ok((module_id, dependencies))
}

/// Analyzes a single Python file with package context and returns module info and dependencies.
pub fn analyze_python_file_with_package(file_path: &Path, project_root: &Path) -> Result<(ModuleIdentifier, Vec<ModuleIdentifier>), Box<dyn std::error::Error>> {
    let python_code = fs::read_to_string(file_path)?;
    let dependencies = extract_module_dependencies_with_context(&python_code, file_path, project_root)?;
    
    // Create module identifier with proper package path
    let module_name = compute_module_name(file_path, project_root)?;
    let module_id = ModuleIdentifier {
        origin: ModuleOrigin::Internal,
        canonical_path: module_name,
    };
    
    Ok((module_id, dependencies))
}

/// Computes the Python module name from file path relative to project root.
/// Uses pyproject.toml package definitions to normalize module names.
/// 
/// Examples:
/// - `/project/main.py` -> `main`
/// - `/project/package/module.py` -> `package.module`
/// - `/project/package/__init__.py` -> `package`
/// - `/project/JOHN/rna/rna/data_processing/binner.py` -> `rna.data_processing.binner`
fn compute_module_name(file_path: &Path, project_root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let relative_path = file_path.strip_prefix(project_root)
        .map_err(|_| format!("File path '{}' is not within project root '{}'", 
                           file_path.display(), project_root.display()))?;
    
    let mut parts = Vec::new();
    
    // Add all directory components from the relative path
    for component in relative_path.components() {
        if let std::path::Component::Normal(name) = component {
            if let Some(name_str) = name.to_str() {
                // For files, check if it's a .py file and handle accordingly
                if name_str.ends_with(".py") {
                    let file_stem = name_str.strip_suffix(".py").unwrap();
                    // Only add if it's not __init__.py
                    if file_stem != "__init__" {
                        parts.push(file_stem.to_string());
                    }
                } else {
                    // It's a directory component
                    parts.push(name_str.to_string());
                }
            }
        }
    }
    
    if parts.is_empty() {
        return Err("Could not determine module name from file path".into());
    }
    
    let full_name = parts.join(".");
    
    // Normalize module name using pyproject.toml package definitions
    normalize_module_name(&full_name, project_root)
}

/// Normalizes a module name using pyproject.toml package definitions.
/// For example, transforms "JOHN.rna.rna.data_processing.binner" -> "rna.data_processing.binner"
fn normalize_module_name(module_name: &str, project_root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    // Get package mappings from pyproject.toml
    let mappings = crate::pyproject::get_package_mappings(project_root)?;
    
    // For each package mapping, check if the module name matches the expected path
    for mapping in &mappings {
        if let Some(from_path) = &mapping.from {
            // Convert "JOHN/rna" to "JOHN.rna"
            let from_dotted = from_path.replace('/', ".");
            
            // Check if module starts with the "from" path
            if module_name.starts_with(&from_dotted) {
                // Strip the "from" path and replace with package name
                // e.g., "JOHN.rna.rna.data_processing.binner" -> "rna.data_processing.binner" 
                if let Some(remainder) = module_name.strip_prefix(&format!("{}.", from_dotted)) {
                    return Ok(format!("{}.{}", mapping.include, remainder));
                } else if module_name == from_dotted {
                    return Ok(mapping.include.clone());
                }
            }
        }
    }
    
    // No normalization needed
    Ok(module_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_temp_python_file(dir: &Path, filename: &str, content: &str) -> PathBuf {
        let file_path = dir.join(filename);
        fs::write(&file_path, content).expect("Failed to write test file");
        file_path
    }

    #[test]
    fn test_analyze_python_directory_empty() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let dir_path = temp_dir.path();
        
        let result = analyze_python_directory(dir_path);
        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_analyze_python_directory_with_python_files() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let dir_path = temp_dir.path();
        
        create_temp_python_file(dir_path, "module1.py", "import os");
        create_temp_python_file(dir_path, "module2.py", "import sys");
        create_temp_python_file(dir_path, "not_python.txt", "not python");
        
        let result = analyze_python_directory(dir_path);
        assert!(result.is_ok());
        let files = result.unwrap();
        
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.file_name().unwrap() == "module1.py"));
        assert!(files.iter().any(|f| f.file_name().unwrap() == "module2.py"));
        
        // Files should be sorted
        assert!(files[0].file_name().unwrap() <= files[1].file_name().unwrap());
    }

    #[test]
    fn test_analyze_python_directory_nonexistent() {
        let nonexistent_path = Path::new("/nonexistent/directory");
        let result = analyze_python_directory(nonexistent_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    #[test]
    fn test_analyze_python_directory_file_not_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let file_path = create_temp_python_file(temp_dir.path(), "test.py", "import os");
        
        let result = analyze_python_directory(&file_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    #[test]
    fn test_analyze_python_file_simple() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let file_path = create_temp_python_file(temp_dir.path(), "test_module.py", "import os\nimport sys");
        
        let result = analyze_python_file(&file_path);
        assert!(result.is_ok());
        
        let (module_id, dependencies) = result.unwrap();
        assert_eq!(module_id.canonical_path, "test_module");
        assert_eq!(module_id.origin, ModuleOrigin::Internal);
        
        assert_eq!(dependencies.len(), 2);
        let dep_names: Vec<&str> = dependencies.iter().map(|d| d.canonical_path.as_str()).collect();
        assert!(dep_names.contains(&"os"));
        assert!(dep_names.contains(&"sys"));
    }

    #[test]
    fn test_analyze_python_file_no_imports() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let file_path = create_temp_python_file(temp_dir.path(), "simple.py", "def hello():\n    return 'world'");
        
        let result = analyze_python_file(&file_path);
        assert!(result.is_ok());
        
        let (module_id, dependencies) = result.unwrap();
        assert_eq!(module_id.canonical_path, "simple");
        assert_eq!(dependencies.len(), 0);
    }

    #[test]
    fn test_analyze_python_file_complex_imports() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let content = r#"
import json
from os import path
from collections import defaultdict
import numpy as np
"#;
        let file_path = create_temp_python_file(temp_dir.path(), "complex.py", content);
        
        let result = analyze_python_file(&file_path);
        assert!(result.is_ok());
        
        let (module_id, dependencies) = result.unwrap();
        assert_eq!(module_id.canonical_path, "complex");
        
        let dep_names: Vec<&str> = dependencies.iter().map(|d| d.canonical_path.as_str()).collect();
        assert!(dep_names.contains(&"json"));
        assert!(dep_names.contains(&"os"));
        assert!(dep_names.contains(&"collections"));
        assert!(dep_names.contains(&"numpy"));
    }

    #[test]
    fn test_analyze_python_file_nonexistent() {
        let nonexistent_path = Path::new("/nonexistent/file.py");
        let result = analyze_python_file(nonexistent_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_directory_dependency_graph_empty() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        let result = build_directory_dependency_graph(temp_dir.path(), None);
        assert!(result.is_ok());
        
        let graph = result.unwrap();
        assert_eq!(graph.module_count(), 0);
        assert_eq!(graph.dependency_count(), 0);
    }

    #[test]
    fn test_build_directory_dependency_graph_single_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        create_temp_python_file(temp_dir.path(), "main.py", "import os\nimport sys");
        
        let result = build_directory_dependency_graph(temp_dir.path(), None);
        assert!(result.is_ok());
        
        let graph = result.unwrap();
        assert_eq!(graph.module_count(), 3); // main + os + sys
        assert_eq!(graph.dependency_count(), 2); // main -> os, main -> sys
    }

    #[test]
    fn test_build_directory_dependency_graph_multiple_files() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        create_temp_python_file(temp_dir.path(), "module1.py", "import json\nfrom os import path");
        create_temp_python_file(temp_dir.path(), "module2.py", "import sys\nimport json");
        create_temp_python_file(temp_dir.path(), "module3.py", "# No imports");
        
        let result = build_directory_dependency_graph(temp_dir.path(), None);
        assert!(result.is_ok());
        
        let graph = result.unwrap();
        
        // Should have: module1, module2, module3, json, os, sys
        assert_eq!(graph.module_count(), 6);
        // Dependencies: module1->json, module1->os, module2->sys, module2->json
        assert_eq!(graph.dependency_count(), 4);
        
        // Verify specific modules exist
        let all_modules: Vec<&str> = graph.all_modules().map(|m| m.canonical_path.as_str()).collect();
        assert!(all_modules.contains(&"module1"));
        assert!(all_modules.contains(&"module2"));
        assert!(all_modules.contains(&"module3"));
        assert!(all_modules.contains(&"json"));
        assert!(all_modules.contains(&"os"));
        assert!(all_modules.contains(&"sys"));
    }

    #[test]
    fn test_build_directory_dependency_graph_with_shared_dependencies() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        create_temp_python_file(temp_dir.path(), "app.py", "import common\nimport json");
        create_temp_python_file(temp_dir.path(), "test.py", "import common\nimport unittest");
        
        let result = build_directory_dependency_graph(temp_dir.path(), None);
        assert!(result.is_ok());
        
        let graph = result.unwrap();
        
        // Should have: app, test, common, json, unittest (5 modules)
        assert_eq!(graph.module_count(), 5);
        // Dependencies: app->common, app->json, test->common, test->unittest (4 deps)
        assert_eq!(graph.dependency_count(), 4);
    }

    #[test]
    fn test_build_directory_dependency_graph_nonexistent_directory() {
        let nonexistent_path = Path::new("/nonexistent/directory");
        let result = build_directory_dependency_graph(nonexistent_path, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_analyze_python_directory_recursive_nested() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let dir_path = temp_dir.path();
        
        // Create nested structure
        fs::create_dir_all(dir_path.join("package/subpackage")).unwrap();
        create_temp_python_file(dir_path, "main.py", "import os");
        create_temp_python_file(&dir_path.join("package"), "module.py", "import sys");
        create_temp_python_file(&dir_path.join("package/subpackage"), "deep.py", "import json");
        create_temp_python_file(&dir_path.join("package"), "__init__.py", "");
        
        let result = analyze_python_directory_recursive(dir_path, None);
        assert!(result.is_ok());
        let files = result.unwrap();
        
        assert_eq!(files.len(), 4);
        let filenames: Vec<String> = files.iter()
            .map(|f| f.strip_prefix(dir_path).unwrap().to_string_lossy().to_string())
            .collect();
        
        assert!(filenames.contains(&"main.py".to_string()));
        assert!(filenames.contains(&"package/module.py".to_string()));
        assert!(filenames.contains(&"package/__init__.py".to_string()));
        assert!(filenames.contains(&"package/subpackage/deep.py".to_string()));
    }

    #[test]
    fn test_compute_module_name() {
        let project_root = Path::new("/project");
        
        // Test simple file
        let file_path = Path::new("/project/main.py");
        assert_eq!(compute_module_name(file_path, project_root).unwrap(), "main");
        
        // Test package module
        let file_path = Path::new("/project/package/module.py");
        assert_eq!(compute_module_name(file_path, project_root).unwrap(), "package.module");
        
        // Test __init__.py
        let file_path = Path::new("/project/package/__init__.py");
        assert_eq!(compute_module_name(file_path, project_root).unwrap(), "package");
        
        // Test deeply nested module
        let file_path = Path::new("/project/deep/nested/module.py");
        assert_eq!(compute_module_name(file_path, project_root).unwrap(), "deep.nested.module");
    }

    #[test]
    fn test_analyze_python_file_with_package() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let project_root = temp_dir.path();
        
        // Create nested structure
        fs::create_dir_all(project_root.join("package")).unwrap();
        let file_path = create_temp_python_file(&project_root.join("package"), "module.py", "import os\nimport sys");
        
        let result = analyze_python_file_with_package(&file_path, project_root);
        assert!(result.is_ok());
        
        let (module_id, dependencies) = result.unwrap();
        assert_eq!(module_id.canonical_path, "package.module");
        assert_eq!(module_id.origin, ModuleOrigin::Internal);
        
        assert_eq!(dependencies.len(), 2);
        let dep_names: Vec<&str> = dependencies.iter().map(|d| d.canonical_path.as_str()).collect();
        assert!(dep_names.contains(&"os"));
        assert!(dep_names.contains(&"sys"));
    }
}