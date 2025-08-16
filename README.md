# Python Dependency Mapper

A fast Rust CLI tool for analyzing Python codebases and mapping module dependencies. Designed for both human engineers and AI coding assistants to understand, maintain, and refactor large Python projects.

## Features

### Core Analysis Commands
- **Impact Analysis** - Identify all modules that depend on a specific module (blast radius)
- **Dependencies** - Show all dependencies of a specific module
- **Circular Dependencies** - Detect and report dependency cycles
- **Dead Code Detection** - Find orphaned modules with no dependents
- **External Dependencies** - Audit external package usage across the codebase
- **Pressure Points** - Identify modules with the highest number of dependents
- **Metrics** - Display overall codebase health indicators

### AI Agent Integration
- **Smart Context Selection** - Curated file lists for AI agents
- **Risk Assessment** - Pre-change complexity analysis
- **Dependency Validation** - Prevent circular dependencies
- **Parallel Work Identification** - Enable multi-agent workflows

### Output Formats
- Human-readable text (default)
- JSON for programmatic processing
- CSV for spreadsheet analysis
- DOT/Graphviz for visualization
- Agent-optimized Markdown
- Cursor IDE context format

## Installation

### Prerequisites
- Rust 1.70+ (uses Rust 2024 edition)
- Python 3.10+ codebase to analyze

### Build from Source
```bash
git clone <repository-url>
cd dep-mapper-rust
cargo build --release
```

The binary will be available at `target/release/dep-mapper`.

## Quick Start

```bash
# Analyze a Python codebase
dep-mapper analyze /path/to/python/project

# Check what modules depend on a specific module
dep-mapper impact src.payments.processor

# Find circular dependencies
dep-mapper cycles

# List external dependencies
dep-mapper external

# Show modules with most dependents (pressure points)
dep-mapper pressure
```

## Usage Examples

### Impact Analysis
```bash
# See what breaks if you change this module
dep-mapper impact src.payments.processor

# Output:
# Modules depending on 'src.payments.processor':
# - src.api.billing (Imports)
# - src.services.subscription (Imports)
# - tests.test_payments (Imports)
# Total: 3 modules affected
```

### Dependencies Inspection
```bash
# See what a module depends on
dep-mapper dependencies src.payments.processor

# Output:
# Dependencies of 'src.payments.processor':
# - stripe (External - Imports)
# - src.models.payment (Internal - Imports)
# - src.utils.validation (Internal - Imports)
# Total: 3 dependencies (1 external, 2 internal)
```

### Architecture Health
```bash
# Check for circular dependencies
dep-mapper cycles

# Get overall metrics
dep-mapper metrics

# Find potential dead code
dep-mapper orphans
```

### AI Agent Integration
```bash
# Get context files for AI agents
dep-mapper agent-context src.payments.processor --format json

# Assess refactoring risk
dep-mapper risk-assess src.models.user

# Validate proposed dependency changes
dep-mapper validate-change src.api.orders src.models.order
```

## Command Reference

| Command | Purpose | Example |
|---------|---------|---------|
| `analyze` | Basic dependency analysis | `dep-mapper analyze .` |
| `impact` | Show dependent modules | `dep-mapper impact MODULE` |
| `dependencies` | Show module dependencies | `dep-mapper dependencies MODULE` |
| `cycles` | Detect circular dependencies | `dep-mapper cycles` |
| `orphans` | Find dead code | `dep-mapper orphans` |
| `external` | List external dependencies | `dep-mapper external` |
| `pressure` | Find high-dependency modules | `dep-mapper pressure` |
| `metrics` | Codebase health overview | `dep-mapper metrics` |

### Global Flags
- `--format FORMAT` - Output format: `text`, `json`, `csv`, `dot`, `agent-md`, `cursor-context`
- `--filter PATTERN` - Include only modules matching pattern
- `--exclude-test` - Exclude test modules
- `--exclude-external` - Exclude external dependencies
- `--limit N` - Limit number of results

## Architecture

### Parser Features
- **Static Analysis**: Uses `rustpython-parser` for AST-based parsing
- **Import Styles**: Supports all Python import patterns:
  - `import module`
  - `from module import name`
  - `from module import *`
  - Nested paths and aliases
- **Original Names**: Extracts original module names (ignores aliases like `import numpy as np`)

### Graph Model
The tool builds a dependency graph with three relationship types:
- **Imports**: Direct import relationships
- **Contains**: Package/module containment
- **IncludedIn**: Reverse containment relationships

### Dependencies
- `rustpython-parser` - Python AST parsing
- `petgraph` - Graph data structures and algorithms
- `clap` - Command line interface
- `walkdir` - File system traversal
- `anyhow` - Error handling
- `serde` - Serialization for JSON output
- `indicatif` - Progress bars

## Development

### Building
```bash
cargo build
cargo run -- analyze /path/to/project
```

### Testing
```bash
cargo test
```

### Code Quality
```bash
cargo clippy
cargo fmt
```

## Use Cases

### For Human Engineers
- **Refactoring**: Understand blast radius before making changes
- **Architecture Review**: Identify circular dependencies and coupling issues
- **Code Cleanup**: Find dead code and unused modules
- **Security Audits**: Track external dependency usage

### For AI Coding Assistants
- **Context Selection**: Get relevant files for code understanding
- **Risk Assessment**: Evaluate change complexity before implementation
- **Dependency Validation**: Prevent architectural violations
- **Parallel Work**: Enable multiple agents to work independently

## Target Scope
- **Codebase Size**: Optimized for Python projects up to ~100k lines
- **Analysis Type**: Static imports only (excludes dynamic imports)
- **Python Version**: Targets Python 3.10+ codebases

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes following the project's principles:
   - Simplicity first (YAGNI)
   - Prefer functions over complex structures
   - Eliminate data redundancy
   - Self-documenting code
4. Run tests and quality checks
5. Submit a pull request

## License

[Add your license information here]