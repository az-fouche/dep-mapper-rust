# Python Dependency Mapper

A Rust CLI tool for analyzing Python codebases and mapping module dependencies.

## Project Scope
- **Target**: Python 3.10 codebases (~100k lines)
- **Analysis**: Static imports only (excludes dynamic imports)
- **Dependencies**: `rustpython-parser`, `clap`, `petgraph`, `walkdir`, `anyhow`
- **Features**: Are described in ENHANCED_FEATURES_SPEC.md

## Development
```bash
cargo build && cargo run
cargo test
cargo clippy && cargo fmt
```

## Parser Features
- Extracts original module names (ignores aliases like `import numpy as np`)
- Supports all import styles: `import module`, `from module import name`, nested paths, star imports
- AST-based parsing via `rustpython-parser`

## Development Principles

### Simplicity First
- **YAGNI**: Avoid over-engineering - build only what's needed
- **Prefer functions**: Use simple functions over complex patterns and structs
- **Minimal code**: The best solution is often the one that requires less code
- **Right-sized solutions**: Don't build classes when functions suffice

### Data Integrity
- **Eliminate redundancy**: When data exists in multiple places, remove one source rather than synchronizing
- **Fail fast**: Return errors for invalid states instead of creating inconsistent data
- **Container structs**: Replace complex return types like `(Vec<ImportInfo>, DependencyGraph, ModuleIdentifier)` with simple containers
- **Reuse types**: Check existing types before creating new enums or structs

### Code Quality
- **Self-documenting code**: Use clear naming and structure instead of explanatory comments
- **Context-only comments**: Only comment on business logic context or decision rationale
- **Scope discipline**: Implement exactly what's requested - no speculative features

### Debugging Methodology
When bugs occur, follow this process:
1. Reproduce the bug reliably
2. Understand expected vs actual behavior
3. Locate where the symptom manifests
4. Identify upstream code entities (functions, data structures)
5. Add strategic logging to understand root cause
6. Once you gathered enough data, formulate one hypothesis
7. Test your hypothesis
8. Design an elegant solution that fixes the root cause
9. Implement and verify the fix
10. Finally, ensure all tests pass so that no new bug is introduced