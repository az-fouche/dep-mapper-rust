use clap::{Parser, Subcommand};
use pydep_mapper::crawler::build_directory_dependency_graph;
use pydep_mapper::tools::agent::print_agent_documentation;
use pydep_mapper::tools::changeset::{analyze_changeset, formatters as changeset_formatters, ChangesetScope};
use pydep_mapper::tools::cycles::{detect_cycles, formatters as cycle_formatters};
use pydep_mapper::tools::dependencies::{analyze_dependencies, formatters as dep_formatters};
use pydep_mapper::tools::diagnose::{analyze_diagnose, formatters as diagnose_formatters};
use pydep_mapper::tools::external::{
    analyze_external_dependencies, formatters as external_formatters,
};
use pydep_mapper::tools::impact::{analyze_impact, formatters};
use pydep_mapper::tools::instability::{analyze_instability, formatters as instability_formatters};
use pydep_mapper::tools::pressure::{analyze_pressure, formatters as pressure_formatters};
use std::path::Path;

#[derive(Parser)]
#[command(name = "pydep-mapper")]
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

    /// Analyze changeset impact and dependencies for safe refactoring
    Changeset {
        /// Module name to analyze for changeset
        module_name: String,
        /// Scope of analysis: affected, dependencies, or both (default: both)
        #[arg(long, default_value = "both")]
        scope: String,
    },

    /// Detect and report circular dependencies in the codebase
    Cycles,

    /// Comprehensive health report of the codebase from a dependency perspective
    Diagnose,

    /// Identify modules with the highest number of dependents (pressure points)
    Pressure,

    /// Identify modules with the highest instability scores (most volatile)
    Instability,

    /// Analyze external dependencies across the codebase with frequency analysis
    External,

    /// Display command documentation optimized for agentic coding workflows
    Agent,
}

fn main() {
    let args = Args::parse();
    let dir_path = Path::new(&args.root);

    // Initialize the pyproject parser once
    pydep_mapper::pyproject::init(dir_path);

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
        Commands::Changeset { module_name, scope } => {
            match run_changeset_analysis(dir_path, &module_name, &scope) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Error running changeset analysis: {}", e);
                }
            }
        }
        Commands::Cycles => match run_cycles_analysis(dir_path) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error running cycles analysis: {}", e);
            }
        },
        Commands::Diagnose => match run_diagnose_analysis(dir_path) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error running diagnose analysis: {}", e);
            }
        },
        Commands::Pressure => match run_pressure_analysis(dir_path) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error running pressure analysis: {}", e);
            }
        },
        Commands::Instability => match run_instability_analysis(dir_path) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error running instability analysis: {}", e);
            }
        },
        Commands::External => match run_external_analysis(dir_path) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error running external analysis: {}", e);
            }
        },
        Commands::Agent => {
            print_agent_documentation();
        }
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

fn run_instability_analysis(dir_path: &Path) -> anyhow::Result<()> {
    // Build the dependency graph
    let graph = build_directory_dependency_graph(dir_path)?;

    // Run instability analysis
    let result = analyze_instability(&graph)?;

    // Output results as text
    print!("{}", instability_formatters::format_text(&result));

    Ok(())
}

fn run_diagnose_analysis(dir_path: &Path) -> anyhow::Result<()> {
    // Build the dependency graph
    let graph = build_directory_dependency_graph(dir_path)?;

    // Run diagnose analysis
    let result = analyze_diagnose(&graph)?;

    // Output results as text
    print!("{}", diagnose_formatters::format_text(&result));

    Ok(())
}

fn run_changeset_analysis(dir_path: &Path, module_name: &str, scope: &str) -> anyhow::Result<()> {
    // Build the dependency graph
    let graph = build_directory_dependency_graph(dir_path)?;

    // Parse scope
    let changeset_scope = ChangesetScope::from_str(scope);

    // Run changeset analysis
    let result = analyze_changeset(&graph, module_name, changeset_scope)?;

    // Output results as text with grouping
    print!("{}", changeset_formatters::format_text_grouped(&result));

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
