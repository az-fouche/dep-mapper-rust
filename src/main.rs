use clap::Parser;
use std::fs;

mod imports;
use imports::extract_imports;

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
            for import in imports {
                println!("  {:?}", import);
            }
        }
        Err(e) => {
            eprintln!("Error parsing Python code: {}", e);
        }
    }
}
