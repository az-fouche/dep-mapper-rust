use crate::graph::{DependencyGraph, DependencyType};
use crate::imports::{ModuleIdentifier, ModuleOrigin, extract_module_deps};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Builds a dependency graph from all Python files in a directory (recursive).
pub fn build_directory_dependency_graph(
    dir_path: &Path,
) -> Result<DependencyGraph> {
    let python_files = analyze_python_directory_recursive(dir_path)?;
    let mut graph = DependencyGraph::new();

    if python_files.is_empty() {
        return Ok(graph);
    }

    let pb = ProgressBar::new(python_files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("##-")
    );
    pb.set_message("Analyzing Python files");

    for file_path in &python_files {
        pb.set_message(format!("Processing {}", file_path.file_name().unwrap_or_default().to_string_lossy()));
        
        match analyze_python_file_with_package(file_path, dir_path) {
            Ok((module_id, dependencies)) => {
                graph.add_module(module_id.clone()); // Ignore duplicates - module might be added as dependency first
                for dep in &dependencies {
                    graph.add_module(dep.clone()); // Ignore duplicates
                    graph.add_dependency(&module_id, dep, DependencyType::Imports)?;
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to analyze '{}': {}",
                    file_path.display(),
                    e
                );
                continue;
            }
        }
        pb.inc(1);
    }

    pb.finish_with_message("Analysis complete");
    Ok(graph)
}

/// Discovers all Python files in a directory (non-recursive).
pub fn analyze_python_directory(
    dir_path: &Path,
) -> Result<Vec<std::path::PathBuf>> {
    if !dir_path.is_dir() {
        return Err(anyhow::anyhow!("Path '{}' is not a directory", dir_path.display()));
    }

    let mut python_files = Vec::new();
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && let Some(extension) = path.extension()
            && extension == "py"
        {
            python_files.push(path);
        }
    }

    // Sort files for consistent output
    python_files.sort();

    Ok(python_files)
}

/// Discovers all Python files in a directory and its subdirectories (recursive).
pub fn analyze_python_directory_recursive(
    dir_path: &Path,
) -> Result<Vec<PathBuf>> {
    if !dir_path.is_dir() {
        return Err(anyhow::anyhow!("Path '{}' is not a directory", dir_path.display()));
    }

    let mut python_files = Vec::new();
    
    let walker = WalkDir::new(dir_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            // Skip directories starting with dot or named 'tests'
            if e.file_type().is_dir()
                && let Some(name) = e.file_name().to_str()
                && (name.starts_with('.') || name == "tests")
            {
                return false;
            }
            e.file_type().is_file()
        });

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    pb.set_message("Discovering Python files...");

    for entry in walker {
        let path = entry.path();
        if let Some(extension) = path.extension()
            && extension == "py"
        {
            python_files.push(path.to_path_buf());
            pb.set_message(format!("Found {} Python files", python_files.len()));
        }
        pb.tick();
    }

    pb.finish_and_clear();

    // Sort files for consistent output
    python_files.sort();

    Ok(python_files)
}

/// Analyzes a single Python file and returns the module identifier and its dependencies.
pub fn analyze_python_file(
    file_path: &Path,
) -> Result<(ModuleIdentifier, Vec<ModuleIdentifier>)> {
    let python_code = fs::read_to_string(file_path)?;
    let dependencies =
        extract_module_deps(&python_code)?;

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
pub fn analyze_python_file_with_package(
    file_path: &Path,
    project_root: &Path,
) -> Result<(ModuleIdentifier, Vec<ModuleIdentifier>)> {
    let python_code = fs::read_to_string(file_path)?;
    let dependencies =
        extract_module_deps(&python_code)?;

    // Create module identifier with proper package path
    let module_name = crate::pyproject::compute_module_name(file_path, project_root)?;
    let module_id = ModuleIdentifier {
        origin: ModuleOrigin::Internal,
        canonical_path: module_name,
    };
    Ok((module_id, dependencies))
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
        let file_path =
            create_temp_python_file(temp_dir.path(), "test_module.py", "import os\nimport sys");

        let result = analyze_python_file(&file_path);
        assert!(result.is_ok());

        let (module_id, dependencies) = result.unwrap();
        assert_eq!(module_id.canonical_path, "test_module");
        assert_eq!(module_id.origin, ModuleOrigin::Internal);

        assert_eq!(dependencies.len(), 2);
        let dep_names: Vec<&str> = dependencies
            .iter()
            .map(|d| d.canonical_path.as_str())
            .collect();
        assert!(dep_names.contains(&"os"));
        assert!(dep_names.contains(&"sys"));
    }

    #[test]
    fn test_analyze_python_file_no_imports() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let file_path = create_temp_python_file(
            temp_dir.path(),
            "simple.py",
            "def hello():\n    return 'world'",
        );

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

        let dep_names: Vec<&str> = dependencies
            .iter()
            .map(|d| d.canonical_path.as_str())
            .collect();
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

        let result = build_directory_dependency_graph(temp_dir.path());
        assert!(result.is_ok());

        let graph = result.unwrap();
        assert_eq!(graph.module_count(), 0);
        assert_eq!(graph.dependency_count(), 0);
    }

    #[test]
    fn test_build_directory_dependency_graph_single_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        create_temp_python_file(temp_dir.path(), "main.py", "import os\nimport sys");

        let result = build_directory_dependency_graph(temp_dir.path());
        assert!(result.is_ok());

        let graph = result.unwrap();
        assert_eq!(graph.module_count(), 3); // main + os + sys
        assert_eq!(graph.dependency_count(), 2); // main -> os, main -> sys
    }

    #[test]
    fn test_build_directory_dependency_graph_multiple_files() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");

        create_temp_python_file(
            temp_dir.path(),
            "module1.py",
            "import json\nfrom os import path",
        );
        create_temp_python_file(temp_dir.path(), "module2.py", "import sys\nimport json");
        create_temp_python_file(temp_dir.path(), "module3.py", "# No imports");

        let result = build_directory_dependency_graph(temp_dir.path());
        assert!(result.is_ok());

        let graph = result.unwrap();

        // Should have: module1, module2, module3, json, os, sys
        assert_eq!(graph.module_count(), 6);
        // Dependencies: module1->json, module1->os, module2->sys, module2->json
        assert_eq!(graph.dependency_count(), 4);

        // Verify specific modules exist
        let all_modules: Vec<&str> = graph
            .all_modules()
            .map(|m| m.canonical_path.as_str())
            .collect();
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

        let result = build_directory_dependency_graph(temp_dir.path());
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
        let result = build_directory_dependency_graph(nonexistent_path);
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
        create_temp_python_file(
            &dir_path.join("package/subpackage"),
            "deep.py",
            "import json",
        );
        create_temp_python_file(&dir_path.join("package"), "__init__.py", "");

        let result = analyze_python_directory_recursive(dir_path);
        assert!(result.is_ok());
        let files = result.unwrap();

        assert_eq!(files.len(), 4);
        let filenames: Vec<String> = files
            .iter()
            .map(|f| {
                f.strip_prefix(dir_path)
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        assert!(filenames.contains(&"main.py".to_string()));
        assert!(filenames.contains(&"package/module.py".to_string()));
        assert!(filenames.contains(&"package/__init__.py".to_string()));
        assert!(filenames.contains(&"package/subpackage/deep.py".to_string()));
    }


    #[test]
    fn test_analyze_python_file_with_package() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let project_root = temp_dir.path();

        // Create nested structure
        fs::create_dir_all(project_root.join("package")).unwrap();
        let file_path = create_temp_python_file(
            &project_root.join("package"),
            "module.py",
            "import os\nimport sys",
        );

        let result = analyze_python_file_with_package(&file_path, project_root);
        assert!(result.is_ok());

        let (module_id, dependencies) = result.unwrap();
        assert_eq!(module_id.canonical_path, "package.module");
        assert_eq!(module_id.origin, ModuleOrigin::Internal);

        assert_eq!(dependencies.len(), 2);
        let dep_names: Vec<&str> = dependencies
            .iter()
            .map(|d| d.canonical_path.as_str())
            .collect();
        assert!(dep_names.contains(&"os"));
        assert!(dep_names.contains(&"sys"));
    }
}
