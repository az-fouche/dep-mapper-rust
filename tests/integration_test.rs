use dep_mapper::{process_python_file, imports::{extract_imports, ModuleOrigin}};

#[test]
fn test_full_workflow_with_test_py() {
    // Process the test Python file using the library function
    let (imports, graph, module_id) = process_python_file("tests/test.py")
        .expect("Should be able to process test.py");

    // Verify we found the expected imports
    assert_eq!(imports.len(), 6);
    
    // Check specific imports
    let import_paths: Vec<String> = imports.iter().map(|import| {
        match import {
            dep_mapper::imports::ImportInfo::Simple(module_id) => module_id.canonical_path.clone(),
            dep_mapper::imports::ImportInfo::From { module, name: _ } => module.canonical_path.clone(),
            dep_mapper::imports::ImportInfo::FromAll(module_id) => module_id.canonical_path.clone(),
        }
    }).collect();
    
    assert!(import_paths.contains(&"os".to_string()));
    assert!(import_paths.contains(&"sys".to_string()));
    assert!(import_paths.contains(&"collections".to_string()));
    assert!(import_paths.contains(&"json".to_string()));
    assert!(import_paths.contains(&"numpy".to_string()));

    // Verify graph state
    assert_eq!(graph.module_count(), 1);
    assert_eq!(graph.dependency_count(), 0);
    
    // Verify module details
    assert_eq!(module_id.canonical_path, "test");
    assert_eq!(module_id.origin, ModuleOrigin::Internal);
    
    // Verify we can retrieve the module from the graph
    let retrieved_module = graph.get_module(&module_id)
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
    
    let imports = extract_imports(python_code)
        .expect("Should parse correctly");
    
    // Check that builtin modules are detected correctly
    for import in imports {
        match import {
            dep_mapper::imports::ImportInfo::Simple(module_id) => {
                match module_id.canonical_path.as_str() {
                    "os" | "sys" => assert_eq!(module_id.origin, ModuleOrigin::Builtin),
                    "numpy" | "requests" => assert_eq!(module_id.origin, ModuleOrigin::Internal), // TODO: Should be External when implemented
                    _ => {}
                }
            }
            dep_mapper::imports::ImportInfo::From { module, name: _ } => {
                if module.canonical_path == "collections" {
                    assert_eq!(module.origin, ModuleOrigin::Builtin);
                }
            }
            _ => {}
        }
    }
}