# Python Dependency Mapper - Rust CLI Tool

## Project Overview
A lightweight CLI tool written in Rust to analyze Python codebases and understand module dependencies. Primary goal is learning Rust while solving a real business problem of untangling legacy Python code.

## Target Codebase
- **Language**: Python 3.10
- **Size**: ~100k lines of code
- **Type**: Large legacy codebase with complex module interactions
- **Analysis Scope**: Static imports only (no dynamic imports)

## Core Features

### Phase 1: Basic Analysis
- **File Scanning**: Recursively scan Python files in a directory
- **Import Extraction**: Parse static imports (`import`, `from ... import`)
- **Module Discovery**: Build catalog of all modules and their locations

### Phase 2: Dependency Graph
- **Dependency Mapping**: Create directed graph of module relationships
- **Circular Dependency Detection**: Identify problematic circular imports
- **Basic CLI Interface**: Command-line interface for querying dependencies

### Phase 3: Dead Code Detection
- **Unused Imports**: Find imports that are never referenced
- **Unused Functions/Classes**: Identify unreferenced definitions
- **Orphaned Modules**: Find modules with no incoming dependencies

### Phase 4: Visualization & Advanced Features
- **Graph Visualization**: Generate DOT files for Graphviz rendering
- **Interactive CLI**: Browse dependencies interactively
- **Export Formats**: JSON output for integration with other tools
- **Metrics**: Calculate coupling metrics, dependency depth, etc.

## Technical Approach

### Key Rust Crates (to explore)
- **`rustpython-parser`**: Python AST parsing
- **`clap`**: Command-line argument parsing
- **`petgraph`**: Graph data structures and algorithms
- **`serde`**: Serialization for JSON export
- **`walkdir`**: Recursive directory traversal
- **`anyhow`**: Error handling

## Development Phases

### Phase 1: Foundation (Learning Rust Basics)
- Set up Rust project with Cargo
- Implement basic file traversal
- Parse Python imports using AST
- Build simple module catalog

### Phase 2: Graph Building (Data Structures)
- Implement dependency graph structure
- Add graph traversal algorithms
- Create basic CLI interface
- Add circular dependency detection

### Phase 3: Analysis (Algorithms)
- Implement dead code detection
- Add usage analysis
- Create report generation
- Add filtering and search capabilities

### Phase 4: Polish (Advanced Features)
- Add visualization export
- Implement interactive mode
- Add comprehensive error handling
- Performance optimization for large codebases

## Learning Objectives

### Rust Concepts to Master
- **Ownership & Borrowing**: File handling and data structure management
- **Error Handling**: Robust parsing and file system operations
- **Pattern Matching**: AST traversal and analysis
- **Iterators**: Efficient data processing
- **Modules & Crates**: Project organization

### Advanced Topics
- **Graph Algorithms**: Dependency analysis and cycle detection
- **CLI Design**: User experience and command structure
- **Performance**: Handling large codebases efficiently
- **Testing**: Unit and integration tests for complex logic

## Success Metrics
1. Successfully parse and analyze the target 100k line codebase
2. Generate meaningful dependency reports
3. Identify actual dead code that can be safely removed
4. Performance: Complete analysis in under 30 seconds
5. Create usable visualizations of module relationships

## Development Commands
```bash
# Build and run
cargo build
cargo run

# Testing
cargo test

# Linting and formatting
cargo clippy
cargo fmt
```

## Implementation Notes

### Current Status (Phase 1)
- ✅ Basic import extraction implemented (`src/imports.rs`)
- ✅ Comprehensive unit tests covering all import types
- ✅ CLI interface for single file analysis (`src/main.rs`)

### Import Parser Features
- **Supported**: `import module`, `from module import name`, `from module import *`
- **Supported**: Nested module paths (`from package.submodule.deep import function`)
- **Supported**: Multiple imports (`import os, sys, json`)
- **Not implemented**: Import aliases (`import numpy as np`) - parser extracts original names only
- **Design decision**: Aliases ignored for architectural analysis (original module names more relevant)

### Test Coverage
Located in `src/imports.rs` following Rust conventions with `#[cfg(test)]`:
- Simple imports, multiple imports, from imports, star imports
- Mixed import scenarios, nested modules, error handling
- Edge cases: empty files, invalid syntax, no imports

### Next Development Priorities
1. **Directory traversal**: Extend from single file to recursive directory scanning
2. **Module catalog**: Build comprehensive inventory of all Python files and their modules
3. **Dependency graph**: Connect imports to actual module definitions
4. **Error handling**: Robust handling of malformed Python files

### Architecture Decisions
- **Import aliases**: Intentionally extract original module names rather than aliases
- **Static analysis only**: No dynamic import resolution (stays within project scope)
- **AST-based parsing**: Using `rustpython-parser` for reliable Python syntax handling