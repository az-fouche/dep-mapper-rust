use clap::Parser;
use dep_mapper::imports::{extract_module_dependencies_with_context, ModuleIdentifier, ModuleOrigin};
use dep_mapper::graph::DependencyGraph;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "dep-mapper")]
#[command(about = "Python dependency mapper")]
struct Args {
    file: String,
}

fn main() {
    let args = Args::parse();
    let file_path = Path::new(&args.file);
    
    match analyze_python_file(file_path) {
        Ok((module_id, dependencies, graph)) => {
            println!("Analyzed file: {}", args.file);
            println!("Module: {} ({:?})", module_id.canonical_path, module_id.origin);
            println!("Found {} dependencies:", dependencies.len());
            for dep in &dependencies {
                println!("  {} ({:?})", dep.canonical_path, dep.origin);
            }
            
            println!("\n--- DependencyGraph Stats ---");
            println!("  Modules: {}", graph.module_count());
            println!("  Dependencies: {}", graph.dependency_count());
        }
        Err(e) => {
            eprintln!("Error processing file '{}': {}", args.file, e);
        }
    }
}

fn analyze_python_file(file_path: &Path) -> Result<(ModuleIdentifier, Vec<ModuleIdentifier>, DependencyGraph), Box<dyn std::error::Error>> {
    // Read the Python file
    let python_code = fs::read_to_string(file_path)?;
    
    // Extract dependencies
    let current_dir = std::env::current_dir()?;
    let dependencies = extract_module_dependencies_with_context(&python_code, file_path, &current_dir)?;
    
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
    
    // Create graph with module and dependencies
    let mut graph = DependencyGraph::new();
    graph.add_module(module_id.clone())?;
    
    for dep in &dependencies {
        graph.add_module(dep.clone()).ok(); // Ignore duplicates
        graph.add_dependency(&module_id, dep)?;
    }
    
    Ok((module_id, dependencies, graph))
}
