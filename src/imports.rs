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

/// Represents different types of Python import statements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportInfo {
    Simple(ModuleIdentifier),
    From { module: ModuleIdentifier, name: String },
    FromAll(ModuleIdentifier),
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

/// Processes a Python AST statement and extracts imports.
fn process_stmt(stmt: &Stmt, imports: &mut Vec<ImportInfo>, source_file: &Path, project_root: &Path) {
    match stmt {
        Stmt::Import(import_stmt) => {
            for alias in &import_stmt.names {
                let module_id = resolve_module_identifier(&alias.name, source_file, project_root);
                imports.push(ImportInfo::Simple(module_id));
            }
        }
        Stmt::ImportFrom(import_from_stmt) => {
            if let Some(module) = &import_from_stmt.module {
                let module_id = resolve_module_identifier(module, source_file, project_root);
                for alias in &import_from_stmt.names {
                    if alias.name.as_str() == "*" {
                        imports.push(ImportInfo::FromAll(module_id.clone()));
                    } else {
                        imports.push(ImportInfo::From {
                            module: module_id.clone(),
                            name: alias.name.to_string(),
                        });
                    }
                }
            }
        }
        _ => {}
    }
}

/// Processes a collection of Python AST statements.
fn process_body(body: &[Stmt], imports: &mut Vec<ImportInfo>, source_file: &Path, project_root: &Path) {
    for stmt in body {
        process_stmt(stmt, imports, source_file, project_root);
    }
}

/// Extracts imports from Python source code with context for resolution.
pub fn extract_imports_with_context(
    python_code: &str, 
    source_file: &Path, 
    project_root: &Path
) -> Result<Vec<ImportInfo>> {
    let ast = parse(python_code, Mode::Module, "<string>")?;
    let mut imports = Vec::new();
    
    match ast {
        Mod::Module(module) => process_body(&module.body, &mut imports, source_file, project_root),
        Mod::Interactive(interactive) => process_body(&interactive.body, &mut imports, source_file, project_root),
        Mod::Expression(_) => {}, // No statements to visit in expression mode
        Mod::FunctionType(_) => {}, // No statements to visit in function type mode
    }
    
    Ok(imports)
}

/// Legacy function for backward compatibility - assumes current directory as context.
pub fn extract_imports(python_code: &str) -> Result<Vec<ImportInfo>> {
    let current_dir = std::env::current_dir()?;
    extract_imports_with_context(python_code, &current_dir.join("<string>"), &current_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_import() {
        let python_code = "import os";
        let imports = extract_imports(python_code).unwrap();
        
        assert_eq!(imports.len(), 1);
        match &imports[0] {
            ImportInfo::Simple(module_id) => assert_eq!(module_id.canonical_path, "os"),
            _ => panic!("Expected Simple import"),
        }
    }

    #[test]
    fn test_multiple_simple_imports() {
        let python_code = "import os, sys, json";
        let imports = extract_imports(python_code).unwrap();
        
        assert_eq!(imports.len(), 3);
        let expected = ["os", "sys", "json"];
        for (i, import) in imports.iter().enumerate() {
            match import {
                ImportInfo::Simple(module_id) => assert_eq!(module_id.canonical_path, expected[i]),
                _ => panic!("Expected Simple import"),
            }
        }
    }

    #[test]
    fn test_from_import() {
        let python_code = "from collections import defaultdict";
        let imports = extract_imports(python_code).unwrap();
        
        assert_eq!(imports.len(), 1);
        match &imports[0] {
            ImportInfo::From { module, name } => {
                assert_eq!(module.canonical_path, "collections");
                assert_eq!(name, "defaultdict");
            },
            _ => panic!("Expected From import"),
        }
    }

    #[test]
    fn test_from_import_multiple() {
        let python_code = "from os.path import join, exists, dirname";
        let imports = extract_imports(python_code).unwrap();
        
        assert_eq!(imports.len(), 3);
        let expected_names = ["join", "exists", "dirname"];
        for (i, import) in imports.iter().enumerate() {
            match import {
                ImportInfo::From { module, name } => {
                    assert_eq!(module.canonical_path, "os.path");
                    assert_eq!(name, expected_names[i]);
                },
                _ => panic!("Expected From import"),
            }
        }
    }

    #[test]
    fn test_from_import_star() {
        let python_code = "from math import *";
        let imports = extract_imports(python_code).unwrap();
        
        assert_eq!(imports.len(), 1);
        match &imports[0] {
            ImportInfo::FromAll(module_id) => assert_eq!(module_id.canonical_path, "math"),
            _ => panic!("Expected FromAll import"),
        }
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
        let imports = extract_imports(python_code).unwrap();
        
        assert_eq!(imports.len(), 7);
        
        match &imports[0] {
            ImportInfo::Simple(module_id) => assert_eq!(module_id.canonical_path, "os"),
            _ => panic!("Expected Simple import for os"),
        }
        
        match &imports[1] {
            ImportInfo::From { module, name } => {
                assert_eq!(module.canonical_path, "sys");
                assert_eq!(name, "argv");
            },
            _ => panic!("Expected From import for sys.argv"),
        }
        
        match &imports[2] {
            ImportInfo::FromAll(module_id) => assert_eq!(module_id.canonical_path, "collections"),
            _ => panic!("Expected FromAll import for collections"),
        }
    }

    #[test]
    fn test_no_imports() {
        let python_code = r#"
def hello():
    print("Hello, world!")

x = 42
"#;
        let imports = extract_imports(python_code).unwrap();
        assert_eq!(imports.len(), 0);
    }

    #[test]
    fn test_invalid_python_code() {
        let python_code = "import os\ndef invalid syntax here";
        let result = extract_imports(python_code);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_code() {
        let python_code = "";
        let imports = extract_imports(python_code).unwrap();
        assert_eq!(imports.len(), 0);
    }

    #[test]
    fn test_nested_from_import() {
        let python_code: &'static str = "from package.submodule.deep import function_name";
        let imports = extract_imports(python_code).unwrap();
        assert_eq!(imports.len(), 1);
        match &imports[0] {
            ImportInfo::From { module, name } => {
                assert_eq!(module.canonical_path, "package.submodule.deep");
                assert_eq!(name, "function_name");
            },
            _ => panic!("Expected From!")
        }
    }

    #[test]
    fn test_import_aliases() {
        let python_code: &'static str = r#"
from collections import defaultdict as dd
import numpy as np
"#;
        let imports = extract_imports(python_code).unwrap();
        assert_eq!(imports.len(), 2);
        match &imports[0] {
            ImportInfo::From { module, name } => {
                assert_eq!(module.canonical_path, "collections");
                assert_eq!(module.origin, ModuleOrigin::Builtin);
                assert_eq!(name, "defaultdict");
            },
            _ => panic!("Expected From!")   
        }
        match &imports[1] {
            ImportInfo::Simple(module_id) => {
                assert_eq!(module_id.canonical_path, "numpy");
                assert_eq!(module_id.origin, ModuleOrigin::Internal); // TODO: Should be External when implemented
            },
            _ => panic!("Expected Simple!")
        }
    }

    #[test]
    fn test_builtin_vs_internal_detection() {
        let python_code = r#"
import os
import sys
import custom_module
"#;
        let imports = extract_imports(python_code).unwrap();
        assert_eq!(imports.len(), 3);
        
        // os should be detected as builtin
        match &imports[0] {
            ImportInfo::Simple(module_id) => {
                assert_eq!(module_id.canonical_path, "os");
                assert_eq!(module_id.origin, ModuleOrigin::Builtin);
            },
            _ => panic!("Expected Simple!")
        }
        
        // sys should be detected as builtin  
        match &imports[1] {
            ImportInfo::Simple(module_id) => {
                assert_eq!(module_id.canonical_path, "sys");
                assert_eq!(module_id.origin, ModuleOrigin::Builtin);
            },
            _ => panic!("Expected Simple!")
        }
        
        // custom_module should be detected as internal (for now)
        match &imports[2] {
            ImportInfo::Simple(module_id) => {
                assert_eq!(module_id.canonical_path, "custom_module");
                assert_eq!(module_id.origin, ModuleOrigin::Internal);
            },
            _ => panic!("Expected Simple!")
        }
    }
}