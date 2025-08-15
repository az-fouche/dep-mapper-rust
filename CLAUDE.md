# Python Dependency Mapper - Rust CLI Tool

Rust CLI tool to analyze Python codebases and understand module dependencies.

## Target
- **Language**: Python 3.10, ~100k lines
- **Code base path:** /path/to/python/project/
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
- ✅ Dependency graph (`src/graph.rs`) with ModuleIdentifier nodes
- ✅ Single file CLI (`src/main.rs`) and integration tests
- **Next**: Directory traversal → Module catalog → Dependency graph

## Parser Design
- Extracts original module names (ignores aliases like `import numpy as np`)
- Supports: `import module`, `from module import name`, nested paths, star imports
- Uses `rustpython-parser` for AST-based parsing

## Coding Style

- **YAGNI**: Avoid over-engineering
- **Function-first**: Use simple functions over complex patterns/structs
- **Question redundancy**: When you see data duplication (same info in multiple places), eliminate one source rather than sync them
- **Fail fast on invariant violations**: Return errors for invalid states instead of silently creating inconsistent data
- **Use containers for convoluted return types**: When functions return complex tuples like `(Vec<ImportInfo>, DependencyGraph, ModuleIdentifier)`, create a simple container struct instead
- **Don't add new types if existing ones suffice**: Before creating a new enum or struct, check if an existing type already captures what you need. Often the core data is already modeled elsewhere
- **Avoid what/how comments**: Don't add comments that explain what the code does or how it works. Code should be self-explanatory through clear naming and structure. Only add comments for business logic context or why decisions were made.
- **Best code is no code at all**: Privilege solutions that require only a small amount of code, or even better that removes some code. It will make our software much easier to maintain in the future.
- **Use the right tool for the right task**: Be careful not to overengineer your solutions, if one or two functions can do the job, no need to add a new class.
- **Debug methodically**: Apply rigorous debugging methodology (1) ensure you are able to reproduce the bug (2) understand what the code is supposed to do, and what it does instead (3) identify the point in the code where the bug symptom occurs (4) identify which code entities (functions, classes, data) are located upstream of the bug (5) implement strategic temporary logging techniques to gather information and understand the root cause of the bug (6) find a way to test and validate your hypothesis (7) figure out an elegant solution that eliminates the bug at its root, not one that hides the symptom or introduces enormous software complexity (8) only then, carry out the implementation (9) verify that the bug is solved, otherwise go back to step (4) and that all tests pass.
- **Do not go overboard**: If the user asks for a feature X, implement the feature X. No need to also implement Y and Z just in case. If the user needs Y and Z, ensure they will ask for it.