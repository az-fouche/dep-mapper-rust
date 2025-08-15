use dep_mapper::graph::DependencyGraph;
use dep_mapper::imports::{ModuleOrigin, extract_module_dependencies_with_context};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[test]
fn test_full_workflow_with_test_py() {
    // Read the test file
    let test_file_path = Path::new("tests/test.py");
    let python_code = fs::read_to_string(test_file_path).expect("Should be able to read test.py");

    // Extract dependencies
    let current_dir = std::env::current_dir().unwrap();
    let dependencies =
        extract_module_dependencies_with_context(&python_code, test_file_path, &current_dir)
            .expect("Should be able to extract dependencies");

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
    let test_module = dep_mapper::imports::ModuleIdentifier {
        origin: ModuleOrigin::Internal,
        canonical_path: "test".to_string(),
    };

    graph.add_module(test_module.clone());
    for dep in &dependencies {
        graph.add_module(dep.clone()); // Ignore duplicates
        graph.add_dependency(&test_module, dep).unwrap();
    }

    // Verify graph state
    assert_eq!(graph.module_count(), 6); // test + 5 dependencies
    assert_eq!(graph.dependency_count(), 5);

    // Verify we can retrieve the module from the graph
    let retrieved_module = graph
        .get_module(&test_module)
        .expect("Should be able to retrieve the module");
    assert_eq!(retrieved_module.canonical_path, "test");
    assert_eq!(retrieved_module.origin, ModuleOrigin::Internal);
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

    let current_dir = std::env::current_dir().unwrap();
    let test_path = current_dir.join("test.py");
    let modules = extract_module_dependencies_with_context(python_code, &test_path, &current_dir)
        .expect("Should parse correctly");

    // Check that builtin modules are detected correctly
    for module in modules {
        match module.canonical_path.as_str() {
            "os" | "sys" | "collections" => assert_eq!(module.origin, ModuleOrigin::Builtin),
            "numpy" | "requests" => assert_eq!(module.origin, ModuleOrigin::External), // Now correctly detected as External
            _ => {}
        }
    }
}
