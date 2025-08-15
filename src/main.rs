use clap::Parser;
use dep_mapper::crawler::build_directory_dependency_graph;
use std::path::Path;

#[derive(Parser)]
#[command(name = "dep-mapper")]
#[command(about = "Python dependency mapper")]
struct Args {
    /// Directory path to analyze for Python files
    #[arg(value_name = "DIRECTORY")]
    dir: String,
}

fn main() {
    let args = Args::parse();
    let dir_path = Path::new(&args.dir);
    
    match build_directory_dependency_graph(dir_path) {
        Ok(graph) => {
            println!("Analyzed directory: {}", args.dir);
            println!("{}", graph.to_string());
        }
        Err(e) => {
            eprintln!("Error processing directory '{}': {}", args.dir, e);
        }
    }
}
