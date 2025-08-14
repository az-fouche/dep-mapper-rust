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

/// Visitor for collecting imports from Python AST.
struct ImportVisitor {
    imports: Vec<ImportInfo>,
}

impl ImportVisitor {
    /// Creates a new ImportVisitor.
    fn new() -> Self {
        Self { imports: Vec::new() }
    }

    /// Visits a Python AST statement and extracts imports.
    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Import(import_stmt) => {
                for alias in &import_stmt.names {
                    self.imports.push(ImportInfo::Simple(alias.name.to_string()));
                }
            }
            Stmt::ImportFrom(import_from_stmt) => {
                if let Some(module) = &import_from_stmt.module {
                    for alias in &import_from_stmt.names {
                        if alias.name.as_str() == "*" {
                            self.imports.push(ImportInfo::FromAll(module.to_string()));
                        } else {
                            self.imports.push(ImportInfo::From {
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

    /// Visits a collection of Python AST statements.
    fn visit_body(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
    }
}

/// Extracts imports from Python source code.
pub fn extract_imports(python_code: &str) -> Result<Vec<ImportInfo>> {
    let ast = parse(python_code, Mode::Module, "<string>")?;
    
    let mut visitor = ImportVisitor::new();
    match ast {
        Mod::Module(module) => visitor.visit_body(&module.body),
        Mod::Interactive(interactive) => visitor.visit_body(&interactive.body),
        Mod::Expression(_) => {}, // No statements to visit in expression mode
        Mod::FunctionType(_) => {}, // No statements to visit in function type mode
    }
    
    Ok(visitor.imports)
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