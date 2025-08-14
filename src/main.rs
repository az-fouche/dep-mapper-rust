use clap::Parser;
use dep_mapper::process_python_file;

#[derive(Parser)]
#[command(name = "dep-mapper")]
#[command(about = "Python dependency mapper")]
struct Args {
    file: String,
}

fn main() {
    let args = Args::parse();
    
    match process_python_file(&args.file) {
        Ok((imports, graph, module_id)) => {
            println!("Found {} imports in '{}':", imports.len(), args.file);
            for import in &imports {
                println!("  {:?}", import);
            }
            
            println!("\n--- DependencyGraph Stats ---");
            println!("  Modules: {}", graph.module_count());
            println!("  Dependencies: {}", graph.dependency_count());
            
            if let Some(module) = graph.get_module(&module_id) {
                println!("\nModule '{}' details:", module_id.canonical_path);
                println!("  Origin: {:?}", module.origin);
                println!("  Path: {}", module.canonical_path);
            }
        }
        Err(e) => {
            eprintln!("Error processing file '{}': {}", args.file, e);
        }
    }
}
