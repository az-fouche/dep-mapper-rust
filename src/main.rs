use clap::Parser;
use dep_mapper::crawler::build_directory_dependency_graph;
use std::path::Path;

#[derive(Parser)]
#[command(name = "dep-mapper")]
#[command(about = "Python dependency mapper")]
struct Args {
    /// Root directory path to analyze for Python files
    #[arg(long, default_value = ".")]
    root: String,
}

fn main() {
    let args = Args::parse();
    let dir_path = Path::new(&args.root);

    // Initialize the pyproject parser once
    dep_mapper::pyproject::init(dir_path);

    match build_directory_dependency_graph(dir_path) {
        Ok(graph) => {
            println!("Analyzed directory: {}", args.root);
            println!("{}", graph);
        }
        Err(e) => {
            eprintln!("Error processing directory '{}': {}", args.root, e);
        }
    }
}
