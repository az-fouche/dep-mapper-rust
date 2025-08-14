# Python Dependency Mapper - Rust CLI Tool

Rust CLI tool to analyze Python codebases and understand module dependencies.

## Target
- **Language**: Python 3.10, ~100k lines
- **Analysis**: Static imports only (no dynamic imports)
- **Crates**: `rustpython-parser`, `clap`, `petgraph`, `walkdir`, `anyhow`

## Development Commands
```bash
cargo build && cargo run
cargo test
cargo clippy && cargo fmt
```

## Current Status
- ✅ Import extraction (`src/imports.rs`) with tests
- ✅ Single file CLI (`src/main.rs`)
- **Next**: Directory traversal → Module catalog → Dependency graph

## Parser Design
- Extracts original module names (ignores aliases like `import numpy as np`)
- Supports: `import module`, `from module import name`, nested paths, star imports
- Uses `rustpython-parser` for AST-based parsing

## Coding Style

- **YAGNI**: Avoid over-engineering
- **Function-first**: Use simple functions over complex patterns/structs