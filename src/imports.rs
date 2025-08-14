use rustpython_parser::ast::{Stmt, Mod};
use rustpython_parser::{parse, Mode};
use anyhow::Result;
use serde::{Serialize, Deserialize};

/// Represents different types of Python import statements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportInfo {
    Simple(String),
    From { module: String, name: String },
    FromAll(String),
}

/// Processes a Python AST statement and extracts imports.
fn process_stmt(stmt: &Stmt, imports: &mut Vec<ImportInfo>) {
    match stmt {
        Stmt::Import(import_stmt) => {
            for alias in &import_stmt.names {
                imports.push(ImportInfo::Simple(alias.name.to_string()));
            }
        }
        Stmt::ImportFrom(import_from_stmt) => {
            if let Some(module) = &import_from_stmt.module {
                for alias in &import_from_stmt.names {
                    if alias.name.as_str() == "*" {
                        imports.push(ImportInfo::FromAll(module.to_string()));
                    } else {
                        imports.push(ImportInfo::From {
                            module: module.to_string(),
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
fn process_body(body: &[Stmt], imports: &mut Vec<ImportInfo>) {
    for stmt in body {
        process_stmt(stmt, imports);
    }
}

/// Extracts imports from Python source code.
pub fn extract_imports(python_code: &str) -> Result<Vec<ImportInfo>> {
    let ast = parse(python_code, Mode::Module, "<string>")?;
    let mut imports = Vec::new();
    
    match ast {
        Mod::Module(module) => process_body(&module.body, &mut imports),
        Mod::Interactive(interactive) => process_body(&interactive.body, &mut imports),
        Mod::Expression(_) => {}, // No statements to visit in expression mode
        Mod::FunctionType(_) => {}, // No statements to visit in function type mode
    }
    
    Ok(imports)
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
            ImportInfo::Simple(name) => assert_eq!(name, "os"),
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
                ImportInfo::Simple(name) => assert_eq!(name, expected[i]),
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
                assert_eq!(module, "collections");
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
                    assert_eq!(module, "os.path");
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
            ImportInfo::FromAll(module) => assert_eq!(module, "math"),
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
            ImportInfo::Simple(name) => assert_eq!(name, "os"),
            _ => panic!("Expected Simple import for os"),
        }
        
        match &imports[1] {
            ImportInfo::From { module, name } => {
                assert_eq!(module, "sys");
                assert_eq!(name, "argv");
            },
            _ => panic!("Expected From import for sys.argv"),
        }
        
        match &imports[2] {
            ImportInfo::FromAll(module) => assert_eq!(module, "collections"),
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
                assert_eq!(module, "package.submodule.deep");
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
                assert_eq!(module, "collections");
                assert_eq!(name, "defaultdict");
            },
            _ => panic!("Expected From!")   
        }
        match &imports[1] {
            ImportInfo::Simple(name) => assert_eq!(name, "numpy"),
            _ => panic!("Expected From!")
        }
    }
}