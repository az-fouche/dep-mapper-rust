# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# Python Dependency Mapper

A Rust CLI tool for analyzing Python codebases and mapping module dependencies.

## Project Scope
- **Target**: Python 3.10 codebases (~100k lines)
- **Analysis**: Static imports only (excludes dynamic imports)
- **Dependencies**: `rustpython-parser`, `clap`, `petgraph`, `walkdir`, `anyhow`
- **Features**: Are described in specs/ENHANCED_FEATURES_SPEC.md

## Development Commands

### Building and Running
```bash
cargo build                    # Build the project
cargo run -- analyze .        # Run analysis on current directory
cargo run -- impact MODULE    # Analyze impact of specific module
cargo run -- cycles           # Detect circular dependencies
cargo run -- pressure         # Find high-pressure modules
cargo run -- external         # Analyze external dependencies
```

### Testing and Code Quality
```bash
cargo test                     # Run all tests
cargo clippy                   # Run linter
cargo fmt                      # Format code
```

### Example Commands for Testing
```bash
# Test with sample Python file
cargo run -- analyze tests/
cargo run -- impact os
cargo run -- dependencies json
```

## Architecture Overview

### Core Components

1. **Parser Layer** (`imports.rs`)
   - Uses `rustpython-parser` for AST-based Python parsing
   - Extracts import statements: `import module`, `from module import name`, star imports
   - Preserves original module names (ignores aliases like `import numpy as np`)
   - Returns `ModuleIdentifier` structs with origin (Internal/External) and canonical paths

2. **Graph Model** (`graph.rs`)
   - Built on `petgraph::DiGraph` for efficient dependency analysis
   - Three relationship types: `Imports`, `Contains`, `IncludedIn`
   - Supports both internal module dependencies and external package tracking
   - Provides traversal methods for impact analysis and cycle detection

3. **File Crawler** (`crawler.rs`)
   - Uses `walkdir` for recursive Python file discovery
   - Builds complete dependency graphs for entire codebases
   - Integrates with `pyproject.toml` parser for package metadata

4. **Analysis Tools** (`tools/` directory)
   - **Impact Analysis**: Find all modules that depend on a target module (blast radius)
   - **Dependencies**: Show what a module depends on
   - **Cycle Detection**: Identify circular dependencies using DFS algorithms
   - **Pressure Points**: Find modules with highest number of dependents
   - **External Dependencies**: Audit external package usage with frequency analysis and manual declarations via `.used-externals.txt`

5. **Common Utilities** (`tools/common.rs`)
   - Shared filtering functions (`filter_hierarchical`, `filter_by_type`)
   - Result formatting utilities for consistent output across tools
   - Dependency type classification helpers

### Command Structure

The CLI uses `clap` subcommands with this pattern:
- `analyze` - Basic dependency graph analysis (being phased out)
- `impact MODULE` - Show what depends on MODULE
- `dependencies MODULE` - Show what MODULE depends on  
- `cycles` - Detect circular dependencies
- `pressure` - Find high-pressure modules (most dependents)
- `external` - List external dependencies with usage stats
- `agent` - Display token-optimized command documentation for agentic coding

All commands support the `--root` flag to specify the directory to analyze (defaults to current directory).

**Note**: When adding/removing CLI commands, always update `src/tools/agent.rs` to keep the agentic documentation current.

### Data Flow

1. **Input**: Python source directory
2. **Parse**: `crawler.rs` discovers files → `imports.rs` extracts dependencies
3. **Model**: `graph.rs` builds dependency relationships
4. **Analyze**: `tools/*` modules perform specific analyses
5. **Output**: Formatted results (text with hierarchical grouping)

### Testing Strategy

- Integration tests in `tests/integration_test.rs` use `tests/test.py` sample file
- Tests verify end-to-end workflow: parsing → graph building → analysis
- Focus on validating dependency extraction accuracy and graph correctness

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

## Parser Features

- Extracts original module names (ignores aliases like `import numpy as np`)
- Supports all import styles: `import module`, `from module import name`, nested paths, star imports
- AST-based parsing via `rustpython-parser`
- Handles both internal module dependencies and external package imports
- Preserves module origin information (Internal vs External)

## External Dependencies Features

### Manual Package Declarations
The external dependencies analysis supports an optional `.used-externals.txt` file located in the same directory as `pyproject.toml`. This allows declaring packages that should be considered "used" even if they're not directly imported in the code.

**File Format:**
- One package name per line
- Comments supported with `#` (full-line or inline)
- Empty lines ignored
- Package names automatically normalized to PyPI conventions (lowercase with hyphens)

**Example `.used-externals.txt`:**
```txt
# Build and deployment tools
setuptools
wheel
docker

# Development tools not directly imported
ruff  # Code formatter
mypy  # Type checker

# Runtime dependencies used via configuration
redis
nginx  # Used in deployment
```

**Integration:**
- Packages from `.used-externals.txt` are merged with code-detected packages
- Manual declarations show as `(declared)` in the used_by_modules list
- Summary shows count of manually declared externals
- Feature works silently when file doesn't exist (fully backward compatible)

## Common Development Tasks

### Adding New Analysis Tools
1. Create new module in `src/tools/`
2. Define analysis function that takes `&DependencyGraph` and returns structured result
3. Create formatter functions in submodule for text output
4. Add to `src/tools/mod.rs` and wire into `main.rs` CLI commands
5. Follow existing patterns in `impact.rs`, `cycles.rs`, etc.
6. **IMPORTANT**: Update `src/tools/agent.rs` documentation to include the new command with examples and descriptions

### Extending Output Formats
- Formatters are organized in submodules within each tool
- Use `format_text_grouped()` pattern for hierarchical output
- Consider adding JSON/CSV formatters following existing patterns

### Graph Algorithm Development
- Leverage `petgraph` for graph traversal and analysis
- Use existing helper functions in `tools/common.rs` for filtering
- Follow DFS-based patterns for dependency analysis