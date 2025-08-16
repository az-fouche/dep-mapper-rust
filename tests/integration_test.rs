use pydep_mapper::graph::{DependencyGraph, DependencyType};
use pydep_mapper::imports::{ModuleOrigin, extract_module_deps};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[test]
fn test_full_workflow_with_test_py() {
    // Read the test file
    let test_file_path = Path::new("tests/test.py");
    let python_code = fs::read_to_string(test_file_path).expect("Should be able to read test.py");

    // Extract dependencies
    let dependencies =
        extract_module_deps(&python_code, None).expect("Should be able to extract dependencies");

    // Verify we found the expected dependencies
    assert_eq!(dependencies.len(), 5);

    let dep_names: HashSet<String> = dependencies
        .iter()
        .map(|m| m.canonical_path.clone())
        .collect();
    assert!(dep_names.contains("os"));
    assert!(dep_names.contains("sys"));
    assert!(dep_names.contains("collections"));
    assert!(dep_names.contains("json"));
    assert!(dep_names.contains("numpy"));

    // Create a graph and verify functionality
    let mut graph = DependencyGraph::new();
    let test_module = pydep_mapper::imports::ModuleIdentifier {
        origin: ModuleOrigin::Internal,
        canonical_path: "test".to_string(),
    };

    graph.add_module(test_module.clone());
    for dep in &dependencies {
        graph.add_module(dep.clone()); // Ignore duplicates
        graph
            .add_dependency(&test_module, dep, DependencyType::Imports)
            .unwrap();
    }

    // Verify graph state
    assert_eq!(graph.module_count(), 6); // test + 5 dependencies
    assert_eq!(graph.dependency_count(), 5);

    // Verify the test module exists in the graph
    let all_modules: Vec<_> = graph.all_modules().collect();
    let test_module_exists = all_modules
        .iter()
        .any(|m| m.canonical_path == "test" && m.origin == ModuleOrigin::Internal);
    assert!(test_module_exists);
}

#[test]
fn test_builtin_vs_external_detection() {
    let python_code = r#"
import os
import sys
import numpy
import requests
from collections import defaultdict
"#;

    let modules = extract_module_deps(python_code, None).expect("Should parse correctly");

    // Check that modules are detected correctly
    for module in modules {
        match module.canonical_path.as_str() {
            "os" | "sys" | "collections" | "numpy" | "requests" => {
                assert_eq!(module.origin, ModuleOrigin::External)
            }
            _ => {}
        }
    }
}
