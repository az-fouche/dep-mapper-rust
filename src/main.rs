use clap::Parser;
use std::fs;
use std::path::PathBuf;
use dep_mapper::imports::extract_imports;
use dep_mapper::graph::{DependencyGraph, ModuleInfo};

#[derive(Parser)]
#[command(name = "dep-mapper")]
#[command(about = "Python dependency mapper")]
struct Args {
    file: String,
}

fn main() {
    let args = Args::parse();
    
    let python_code = match fs::read_to_string(&args.file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", args.file, e);
            return;
        }
    };

    match extract_imports(&python_code) {
        Ok(imports) => {
            println!("Found {} imports in '{}':", imports.len(), args.file);
            for import in &imports {
                println!("  {:?}", import);
            }
            
            // Test the graph functionality
            println!("\n--- Testing DependencyGraph ---");
            let mut graph = DependencyGraph::new();
            
            // Create a module info for the analyzed file
            let module_name = PathBuf::from(&args.file)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
                
            let module_info = ModuleInfo {
                file_path: PathBuf::from(&args.file),
                module_name: module_name.clone(),
                imports: imports.clone(),
            };
            
            graph.add_module(module_info);
            
            println!("Graph stats:");
            println!("  Modules: {}", graph.module_count());
            println!("  Dependencies: {}", graph.dependency_count());
            
            if let Some(module) = graph.get_module(&module_name) {
                println!("\nModule '{}' details:", module_name);
                println!("  File: {:?}", module.file_path);
                println!("  Imports: {}", module.imports.len());
            }
        }
        Err(e) => {
            eprintln!("Error parsing Python code: {}", e);
        }
    }
}
