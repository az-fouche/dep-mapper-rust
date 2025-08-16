use clap::{Parser, Subcommand};
use dep_mapper::crawler::build_directory_dependency_graph;
use dep_mapper::tools::cycles::{detect_cycles, formatters as cycle_formatters};
use dep_mapper::tools::dependencies::{analyze_dependencies, formatters as dep_formatters};
use dep_mapper::tools::external::{
    analyze_external_dependencies, formatters as external_formatters,
};
use dep_mapper::tools::impact::{analyze_impact, formatters};
use dep_mapper::tools::pressure::{analyze_pressure, formatters as pressure_formatters};
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

    /// Show all dependencies of the specified module
    Dependencies {
        /// Module name to analyze for dependencies
        module_name: String,
    },

    /// Detect and report circular dependencies in the codebase
    Cycles,

    /// Identify modules with the highest number of dependents (pressure points)
    Pressure,

    /// Analyze external dependencies across the codebase with frequency analysis
    External,
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
        Commands::Dependencies { module_name } => {
            match run_dependencies_analysis(dir_path, &module_name) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Error running dependencies analysis: {}", e);
                }
            }
        }
        Commands::Cycles => match run_cycles_analysis(dir_path) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error running cycles analysis: {}", e);
            }
        },
        Commands::Pressure => match run_pressure_analysis(dir_path) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error running pressure analysis: {}", e);
            }
        },
        Commands::External => match run_external_analysis(dir_path) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error running external analysis: {}", e);
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

fn run_dependencies_analysis(dir_path: &Path, module_name: &str) -> anyhow::Result<()> {
    // Build the dependency graph
    let graph = build_directory_dependency_graph(dir_path)?;

    // Run dependencies analysis
    let result = analyze_dependencies(&graph, module_name)?;

    // Output results as text with prefix grouping
    print!("{}", dep_formatters::format_text_grouped(&result));

    Ok(())
}

fn run_cycles_analysis(dir_path: &Path) -> anyhow::Result<()> {
    // Build the dependency graph
    let graph = build_directory_dependency_graph(dir_path)?;

    // Run cycle detection
    let result = detect_cycles(&graph)?;

    // Output results as text with prefix grouping
    print!("{}", cycle_formatters::format_text_grouped(&result));

    Ok(())
}

fn run_pressure_analysis(dir_path: &Path) -> anyhow::Result<()> {
    // Build the dependency graph
    let graph = build_directory_dependency_graph(dir_path)?;

    // Run pressure analysis
    let result = analyze_pressure(&graph)?;

    // Output results as text
    print!("{}", pressure_formatters::format_text(&result));

    Ok(())
}

fn run_external_analysis(dir_path: &Path) -> anyhow::Result<()> {
    // Build the dependency graph
    let graph = build_directory_dependency_graph(dir_path)?;

    // Run external dependencies analysis
    let result = analyze_external_dependencies(&graph)?;

    // Output results as text with grouping
    print!("{}", external_formatters::format_text_grouped(&result));

    Ok(())
}
