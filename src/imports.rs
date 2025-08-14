use rustpython_parser::ast::{Stmt, Mod};
use rustpython_parser::{parse, Mode};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::collections::HashSet;

/// Represents the origin type of a Python module.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleOrigin {
    Builtin,    // Python standard library modules
    External,   // Third-party packages from site-packages
    Internal,   // Project modules within the same codebase
}

/// Unique identifier for a Python module.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleIdentifier {
    pub origin: ModuleOrigin,
    pub canonical_path: String,
}


/// Gets Python stdlib module names from sys.stdlib_module_names
fn get_stdlib_modules() -> Result<HashSet<String>> {
    use std::process::Command;
    
    let output = Command::new("python3")
        .args(["-c", "import sys; print('\n'.join(sorted(sys.stdlib_module_names)))"])
        .output()?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to get stdlib modules: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    let modules = String::from_utf8(output.stdout)?
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    Ok(modules)
}

/// Cached stdlib module names
static STDLIB_MODULES: std::sync::OnceLock<HashSet<String>> = std::sync::OnceLock::new();

/// Check if a module is part of Python's standard library
fn is_stdlib_module(module_name: &str) -> bool {
    let stdlib = STDLIB_MODULES.get_or_init(|| {
        get_stdlib_modules().unwrap_or_else(|_| {
            // Fallback to minimal set if python3 command fails
            HashSet::from([
                "os".to_string(), "sys".to_string(), "json".to_string(),
                "re".to_string(), "math".to_string(), "collections".to_string(),
            ])
        })
    });
    
    // Check the top-level module name (e.g., "os" from "os.path")
    let top_level = module_name.split('.').next().unwrap_or(module_name);
    stdlib.contains(top_level)
}

/// Resolves a module name to a ModuleIdentifier.
fn resolve_module_identifier(module_name: &str, _source_file: &Path, _project_root: &Path) -> ModuleIdentifier {
    let origin = if is_stdlib_module(module_name) {
        ModuleOrigin::Builtin
    } else {
        // For now, treat everything else as Internal
        // TODO: Add External detection and relative import resolution
        ModuleOrigin::Internal
    };
    
    ModuleIdentifier {
        origin,
        canonical_path: module_name.to_string(),
    }
}

/// Processes a Python AST statement and extracts module dependencies.
fn process_stmt(stmt: &Stmt, modules: &mut HashSet<ModuleIdentifier>, source_file: &Path, project_root: &Path) {
    match stmt {
        Stmt::Import(import_stmt) => {
            for alias in &import_stmt.names {
                let module_id = resolve_module_identifier(&alias.name, source_file, project_root);
                modules.insert(module_id);
            }
        }
        Stmt::ImportFrom(import_from_stmt) => {
            if let Some(module) = &import_from_stmt.module {
                let module_id = resolve_module_identifier(module, source_file, project_root);
                modules.insert(module_id);
            }
        }
        _ => {}
    }
}

/// Processes a collection of Python AST statements.
fn process_body(body: &[Stmt], modules: &mut HashSet<ModuleIdentifier>, source_file: &Path, project_root: &Path) {
    for stmt in body {
        process_stmt(stmt, modules, source_file, project_root);
    }
}

/// Extracts module dependencies from Python source code with context for resolution.
pub fn extract_module_dependencies_with_context(
    python_code: &str, 
    source_file: &Path, 
    project_root: &Path
) -> Result<Vec<ModuleIdentifier>> {
    let ast = parse(python_code, Mode::Module, "<string>")?;
    let mut modules = HashSet::new();
    
    match ast {
        Mod::Module(module) => process_body(&module.body, &mut modules, source_file, project_root),
        Mod::Interactive(interactive) => process_body(&interactive.body, &mut modules, source_file, project_root),
        Mod::Expression(_) => {}, // No statements to visit in expression mode
        Mod::FunctionType(_) => {}, // No statements to visit in function type mode
    }
    
    Ok(modules.into_iter().collect())
}

/// Legacy function for backward compatibility - assumes current directory as context.
pub fn extract_module_dependencies(python_code: &str) -> Result<Vec<ModuleIdentifier>> {
    let current_dir = std::env::current_dir()?;
    extract_module_dependencies_with_context(python_code, &current_dir.join("<string>"), &current_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_import() {
        let python_code = "import os";
        let modules = extract_module_dependencies(python_code).unwrap();
        
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].canonical_path, "os");
    }

    #[test]
    fn test_multiple_simple_imports() {
        let python_code = "import os, sys, json";
        let modules = extract_module_dependencies(python_code).unwrap();
        
        assert_eq!(modules.len(), 3);
        let module_names: HashSet<String> = modules.iter().map(|m| m.canonical_path.clone()).collect();
        assert!(module_names.contains("os"));
        assert!(module_names.contains("sys"));
        assert!(module_names.contains("json"));
    }

    #[test]
    fn test_from_import() {
        let python_code = "from collections import defaultdict";
        let modules = extract_module_dependencies(python_code).unwrap();
        
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].canonical_path, "collections");
    }

    #[test]
    fn test_from_import_multiple() {
        let python_code = "from os.path import join, exists, dirname";
        let modules = extract_module_dependencies(python_code).unwrap();
        
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].canonical_path, "os.path");
    }

    #[test]
    fn test_from_import_star() {
        let python_code = "from math import *";
        let modules = extract_module_dependencies(python_code).unwrap();
        
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].canonical_path, "math");
    }

    #[test]
    fn test_mixed_imports() {
        let python_code = r#"
import os
from sys import argv
from collections import *
import json, re
from typing import List, Dict
"#;
        let modules = extract_module_dependencies(python_code).unwrap();
        
        assert_eq!(modules.len(), 6);
        let module_names: HashSet<String> = modules.iter().map(|m| m.canonical_path.clone()).collect();
        assert!(module_names.contains("os"));
        assert!(module_names.contains("sys"));
        assert!(module_names.contains("collections"));
        assert!(module_names.contains("json"));
        assert!(module_names.contains("re"));
        assert!(module_names.contains("typing"));
    }

    #[test]
    fn test_no_imports() {
        let python_code = r#"
def hello():
    print("Hello, world!")

x = 42
"#;
        let modules = extract_module_dependencies(python_code).unwrap();
        assert_eq!(modules.len(), 0);
    }

    #[test]
    fn test_invalid_python_code() {
        let python_code = "import os\ndef invalid syntax here";
        let result = extract_module_dependencies(python_code);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_code() {
        let python_code = "";
        let modules = extract_module_dependencies(python_code).unwrap();
        assert_eq!(modules.len(), 0);
    }

    #[test]
    fn test_nested_from_import() {
        let python_code: &'static str = "from package.submodule.deep import function_name";
        let modules = extract_module_dependencies(python_code).unwrap();
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].canonical_path, "package.submodule.deep");
    }

    #[test]
    fn test_import_aliases() {
        let python_code: &'static str = r#"
from collections import defaultdict as dd
import numpy as np
"#;
        let modules = extract_module_dependencies(python_code).unwrap();
        assert_eq!(modules.len(), 2);
        let module_names: HashSet<String> = modules.iter().map(|m| m.canonical_path.clone()).collect();
        assert!(module_names.contains("collections"));
        assert!(module_names.contains("numpy"));
        
        // Check origins
        let collections_module = modules.iter().find(|m| m.canonical_path == "collections").unwrap();
        assert_eq!(collections_module.origin, ModuleOrigin::Builtin);
        
        let numpy_module = modules.iter().find(|m| m.canonical_path == "numpy").unwrap();
        assert_eq!(numpy_module.origin, ModuleOrigin::Internal); // TODO: Should be External when implemented
    }

    #[test]
    fn test_builtin_vs_internal_detection() {
        let python_code = r#"
import os
import sys
import custom_module
"#;
        let modules = extract_module_dependencies(python_code).unwrap();
        assert_eq!(modules.len(), 3);
        let module_names: HashSet<String> = modules.iter().map(|m| m.canonical_path.clone()).collect();
        assert!(module_names.contains("os"));
        assert!(module_names.contains("sys"));
        assert!(module_names.contains("custom_module"));
        
        // os should be detected as builtin
        let os_module = modules.iter().find(|m| m.canonical_path == "os").unwrap();
        assert_eq!(os_module.origin, ModuleOrigin::Builtin);
        
        // sys should be detected as builtin  
        let sys_module = modules.iter().find(|m| m.canonical_path == "sys").unwrap();
        assert_eq!(sys_module.origin, ModuleOrigin::Builtin);
        
        // custom_module should be detected as internal (for now)
        let custom_module = modules.iter().find(|m| m.canonical_path == "custom_module").unwrap();
        assert_eq!(custom_module.origin, ModuleOrigin::Internal);
    }
}