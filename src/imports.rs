use anyhow::Result;
use rustpython_parser::ast::{Mod, Stmt};
use rustpython_parser::{Mode, parse};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Represents the origin type of a Python module.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleOrigin {
    External, // Standard library and third-party packages
    Internal, // Project modules within the same codebase
}

/// Unique identifier for a Python module.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleIdentifier {
    pub origin: ModuleOrigin,
    pub canonical_path: String,
}

/// Extracts the root module name from a dotted module path.
fn extract_root_module(module_name: &str) -> &str {
    module_name.split('.').next().unwrap_or(module_name)
}

/// Resolves a module name to a ModuleIdentifier.
fn resolve_module_identifier(module_name: &str) -> ModuleIdentifier {
    let origin = if crate::pyproject::is_internal_module(module_name) {
        ModuleOrigin::Internal
    } else {
        ModuleOrigin::External
    };

    let canonical_path = match origin {
        ModuleOrigin::Internal => crate::pyproject::normalize_module_name(module_name)
            .unwrap_or_else(|_| module_name.to_string()),
        _ => extract_root_module(module_name).to_string(),
    };

    ModuleIdentifier {
        origin,
        canonical_path,
    }
}

/// Processes a Python AST statement and extracts module dependencies.
fn process_stmt(stmt: &Stmt, modules: &mut HashSet<ModuleIdentifier>) {
    match stmt {
        Stmt::Import(import_stmt) => {
            for alias in &import_stmt.names {
                let module_id = resolve_module_identifier(&alias.name);
                modules.insert(module_id);
            }
        }
        Stmt::ImportFrom(import_from_stmt) => {
            if let Some(module) = &import_from_stmt.module {
                let module_id = resolve_module_identifier(module);
                modules.insert(module_id);
            }
        }
        _ => {}
    }
}

/// Processes a collection of Python AST statements.
fn process_body(body: &[Stmt], modules: &mut HashSet<ModuleIdentifier>) {
    for stmt in body {
        process_stmt(stmt, modules);
    }
}

/// Extracts module dependencies from Python source code with context for resolution.
pub fn extract_module_deps(python_code: &str) -> Result<Vec<ModuleIdentifier>> {
    let ast = parse(python_code, Mode::Module, "<string>")?;
    let mut modules = HashSet::new();

    match ast {
        Mod::Module(module) => process_body(&module.body, &mut modules),
        Mod::Interactive(interactive) => process_body(&interactive.body, &mut modules),
        Mod::Expression(_) => {} // No statements to visit in expression mode
        Mod::FunctionType(_) => {} // No statements to visit in function type mode
    }

    Ok(modules.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_import() {
        let python_code = "import os";
        let modules = extract_module_deps(python_code).unwrap();

        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].canonical_path, "os");
    }

    #[test]
    fn test_multiple_simple_imports() {
        let python_code = "import os, sys, json";
        let modules = extract_module_deps(python_code).unwrap();

        assert_eq!(modules.len(), 3);
        let module_names: HashSet<String> =
            modules.iter().map(|m| m.canonical_path.clone()).collect();
        assert!(module_names.contains("os"));
        assert!(module_names.contains("sys"));
        assert!(module_names.contains("json"));
    }

    #[test]
    fn test_from_import() {
        let python_code = "from collections import defaultdict";
        let modules = extract_module_deps(python_code).unwrap();

        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].canonical_path, "collections");
    }

    #[test]
    fn test_from_import_multiple() {
        let python_code = "from os.path import join, exists, dirname";
        let modules = extract_module_deps(python_code).unwrap();

        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].canonical_path, "os");
    }

    #[test]
    fn test_from_import_star() {
        let python_code = "from math import *";
        let modules = extract_module_deps(python_code).unwrap();

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
        let modules = extract_module_deps(python_code).unwrap();

        assert_eq!(modules.len(), 6);
        let module_names: HashSet<String> =
            modules.iter().map(|m| m.canonical_path.clone()).collect();
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
        let modules = extract_module_deps(python_code).unwrap();
        assert_eq!(modules.len(), 0);
    }

    #[test]
    fn test_invalid_python_code() {
        let python_code = "import os\ndef invalid syntax here";
        let result = extract_module_deps(python_code);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_code() {
        let python_code = "";
        let modules = extract_module_deps(python_code).unwrap();
        assert_eq!(modules.len(), 0);
    }

    #[test]
    fn test_nested_from_import() {
        let python_code: &'static str = "from package.submodule.deep import function_name";
        let modules = extract_module_deps(python_code).unwrap();
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].canonical_path, "package");
    }

    #[test]
    fn test_import_aliases() {
        let python_code: &'static str = r#"
from collections import defaultdict as dd
import numpy as np
"#;
        let modules = extract_module_deps(python_code).unwrap();
        assert_eq!(modules.len(), 2);
        let module_names: HashSet<String> =
            modules.iter().map(|m| m.canonical_path.clone()).collect();
        assert!(module_names.contains("collections"));
        assert!(module_names.contains("numpy"));

        // Check origins
        let collections_module = modules
            .iter()
            .find(|m| m.canonical_path == "collections")
            .unwrap();
        assert_eq!(collections_module.origin, ModuleOrigin::External);

        let numpy_module = modules
            .iter()
            .find(|m| m.canonical_path == "numpy")
            .unwrap();
        assert_eq!(numpy_module.origin, ModuleOrigin::External);
    }

    #[test]
    fn test_builtin_vs_internal_detection() {
        let python_code = r#"
import os
import sys
import custom_module
"#;
        let modules = extract_module_deps(python_code).unwrap();
        assert_eq!(modules.len(), 3);
        let module_names: HashSet<String> =
            modules.iter().map(|m| m.canonical_path.clone()).collect();
        assert!(module_names.contains("os"));
        assert!(module_names.contains("sys"));
        assert!(module_names.contains("custom_module"));

        // os should be detected as external
        let os_module = modules.iter().find(|m| m.canonical_path == "os").unwrap();
        assert_eq!(os_module.origin, ModuleOrigin::External);

        // sys should be detected as external
        let sys_module = modules.iter().find(|m| m.canonical_path == "sys").unwrap();
        assert_eq!(sys_module.origin, ModuleOrigin::External);

        // custom_module should be detected as external (since no pyproject.toml in test)
        let custom_module = modules
            .iter()
            .find(|m| m.canonical_path == "custom_module")
            .unwrap();
        assert_eq!(custom_module.origin, ModuleOrigin::External);
    }

    #[test]
    fn test_root_module_extraction() {
        let python_code = r#"
import os.path
from collections.abc import Mapping
import numpy.testing.utils
from requests.auth import HTTPBasicAuth
"#;
        let modules = extract_module_deps(python_code).unwrap();

        assert_eq!(modules.len(), 4);
        let module_names: HashSet<String> =
            modules.iter().map(|m| m.canonical_path.clone()).collect();

        // All should be normalized to root modules
        assert!(module_names.contains("os"));
        assert!(module_names.contains("collections"));
        assert!(module_names.contains("numpy"));
        assert!(module_names.contains("requests"));

        // Verify they don't contain full paths
        assert!(!module_names.contains("os.path"));
        assert!(!module_names.contains("collections.abc"));
        assert!(!module_names.contains("numpy.testing.utils"));
        assert!(!module_names.contains("requests.auth"));
    }
}
