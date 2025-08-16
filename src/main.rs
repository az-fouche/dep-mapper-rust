use clap::{Parser, Subcommand};
use dep_mapper::crawler::build_directory_dependency_graph;
use dep_mapper::tools::impact::{analyze_impact, formatters};
use std::path::Path;

#[derive(Parser)]
#[command(name = "dep-mapper")]
#[command(about = "Python dependency mapper and analyzer")]
struct Args {
    /// Root directory path to analyze for Python files
    #[arg(long, default_value = ".")]
    root: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze the entire codebase and display dependency graph
    Analyze,

    /// Identify all modules that depend on the specified module (blast radius analysis)
    Impact {
        /// Module name to analyze for impact
        module_name: String,
    },
}

fn main() {
    let args = Args::parse();
    let dir_path = Path::new(&args.root);

    // Initialize the pyproject parser once
    dep_mapper::pyproject::init(dir_path);

    match args.command {
        Commands::Analyze => match build_directory_dependency_graph(dir_path) {
            Ok(graph) => {
                println!("Analyzed directory: {}", args.root);
                println!("{}", graph);
            }
            Err(e) => {
                eprintln!("Error processing directory '{}': {}", args.root, e);
            }
        },
        Commands::Impact { module_name } => match run_impact_analysis(dir_path, &module_name) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error running impact analysis: {}", e);
            }
        },
    }
}

fn run_impact_analysis(dir_path: &Path, module_name: &str) -> anyhow::Result<()> {
    // Build the dependency graph
    let graph = build_directory_dependency_graph(dir_path)?;

    // Run impact analysis
    let result = analyze_impact(&graph, module_name)?;

    // Output results as text with prefix grouping
    print!("{}", formatters::format_text_grouped(&result));

    Ok(())
}
