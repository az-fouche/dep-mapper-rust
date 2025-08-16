# External Dependencies Audit Specification

## Overview
Implementation specification for the `pydep-mapper external` command (Feature 1.5) to audit external Python dependencies with three key analyses:
1. **Frequency analysis**: Most/least used external dependencies  
2. **Undeclared detection**: Dependencies used in code but missing from pyproject.toml
3. **Unused detection**: Dependencies declared in pyproject.toml but not used in code

## Motivation
Based on analysis of the ScientaLab reference codebase (~100k lines, 100+ dependencies), we need:
- **Security auditing**: Know which external packages we actually depend on
- **Dependency cleanup**: Remove unused declared dependencies
- **Compliance checking**: Ensure all used dependencies are properly declared
- **Architecture insights**: Understand coupling to external packages

## Current System Foundation
The existing dependency mapper provides:
- Static import analysis via `rustpython-parser`
- Internal/External module classification using pyproject.toml
- External dependency normalization to root modules (e.g., `numpy.testing` → `numpy`)
- Dependency graph with relationship tracking

## Architecture & Implementation

### 1. PyProject.toml Parser Enhancement (`src/pyproject.rs`)

**Extend existing `PyProjectParser`** to extract declared dependencies:

```rust
impl PyProjectParser {
    /// Extract all declared dependencies from pyproject.toml
    pub fn get_all_declared_dependencies(&self) -> Result<Vec<String>> {
        // Parse [tool.poetry.dependencies] 
        // Parse all [tool.poetry.group.*.dependencies]
        // Normalize dependency names (handle git URLs, paths, versions)
    }
    
    /// Normalize dependency name from complex specs
    pub fn normalize_dependency_name(dep_spec: &str) -> String {
        // Handle: "torch = { version = "2.3.0"}"
        // Handle: "flash-attn = { url = "https://..." }"
        // Handle: "john = { path = "JOHN", develop = true }"
    }
}
```

**Handle complex dependency specifications** from ScientaLab's pyproject.toml:
- Version constraints: `numpy = "^1.24.3"`
- Git dependencies: `geneformer = {git = "https://...", rev = "..."}`
- Local path dependencies: `john = { path = "JOHN", develop = true }`
- URL dependencies: `flash-attn = { url = "https://..." }`
- Optional groups: `[tool.poetry.group.eva.dependencies]`

### 2. External Dependencies Analysis Module (`src/tools/external.rs`)

**Create new analysis module** following existing patterns:

```rust
#[derive(Debug)]
pub struct ExternalAnalysisResult {
    pub frequency_analysis: Vec<DependencyUsage>,
    pub undeclared_dependencies: Vec<String>,
    pub unused_dependencies: Vec<String>,
    pub summary: ExternalDependencySummary,
}

#[derive(Debug)]
pub struct DependencyUsage {
    pub package_name: String,
    pub usage_count: usize,
    pub used_by_modules: Vec<String>,
}

#[derive(Debug)]
pub struct ExternalDependencySummary {
    pub total_used_packages: usize,
    pub total_declared_dependencies: usize,
    pub undeclared_count: usize,
    pub unused_count: usize,
}

/// Main analysis function
pub fn analyze_external_dependencies(graph: &DependencyGraph) -> Result<ExternalAnalysisResult>
```

**Core analysis logic**:

1. **Frequency Analysis**:
   - Filter graph for external modules only
   - Count usage across internal modules
   - Sort by usage frequency (descending)
   - Group by usage tiers (high/medium/low)

2. **Undeclared Detection**:
   - Extract used external packages from graph
   - Extract declared dependencies from pyproject.toml
   - Set difference: `used_packages - declared_packages`

3. **Unused Detection**:
   - Extract declared dependencies from pyproject.toml
   - Extract used external packages from graph
   - Set difference: `declared_packages - used_packages`

### 3. CLI Integration (`src/main.rs`)

**Add new subcommand**:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands
    
    /// List all external dependencies across the codebase with usage analysis
    External,
}

fn run_external_analysis(dir_path: &Path) -> anyhow::Result<()> {
    let graph = build_directory_dependency_graph(dir_path)?;
    let result = analyze_external_dependencies(&graph)?;
    print!("{}", external_formatters::format_text_grouped(&result));
    Ok(())
}
```

### 4. Output Formatters (`src/tools/external/formatters.rs`)

**Text formatter** matching specification:

```rust
pub fn format_text_grouped(result: &ExternalAnalysisResult) -> String {
    // Frequency analysis with tiered grouping
    // Undeclared dependencies section
    // Unused dependencies section  
    // Summary statistics
}
```

**Expected output format**:
```
External Dependencies Analysis:

=== Frequency Analysis ===
High usage (10+ modules):
  numpy (used by 15 modules)
  pandas (used by 12 modules)

Medium usage (5-9 modules):  
  matplotlib (used by 8 modules)
  torch (used by 6 modules)

Low usage (1-4 modules):
  requests (used by 3 modules)
  boto3 (used by 1 module)

=== Undeclared Dependencies ===
The following packages are used in code but not declared in pyproject.toml:
  psutil (used by 2 modules)
  click (used by 1 module)

=== Unused Dependencies ===
The following packages are declared in pyproject.toml but not used in code:
  pytest-asyncio
  black

=== Summary ===
Total external packages used: 45
Total declared dependencies: 47
Undeclared dependencies: 2
Unused dependencies: 2
```

## Implementation Steps

### Phase 1: PyProject Enhancement
1. Extend `PyProjectParser::load_package_info()` to also load dependencies
2. Implement `get_all_declared_dependencies()` with TOML parsing
3. Implement `normalize_dependency_name()` for complex specs
4. Add comprehensive tests covering ScientaLab patterns

### Phase 2: Core Analysis Logic  
1. Create `src/tools/external.rs` module
2. Implement frequency analysis using graph traversal
3. Implement undeclared/unused detection using set operations
4. Add result data structures

### Phase 3: CLI Integration
1. Add `External` command to main.rs
2. Implement `run_external_analysis()` function
3. Wire up with existing graph building pipeline

### Phase 4: Output Formatting
1. Create `src/tools/external/formatters.rs`
2. Implement text formatter with tiered grouping
3. Structure for future JSON/CSV formats

### Phase 5: Testing & Validation
1. Unit tests for each component
2. Integration test using ScientaLab as reference
3. Edge case testing (missing pyproject.toml, etc.)

## Edge Cases & Considerations

### PyProject.toml Parsing
- **Missing pyproject.toml**: Return empty declared dependencies list
- **Non-Poetry projects**: Support setup.py parsing in future
- **Case sensitivity**: Normalize package names (e.g., `PyYAML` vs `pyyaml`)
- **Complex specs**: Extract base package name from complex dependency specs

### Dependency Name Normalization
- **Import vs package names**: Map import names to package names (e.g., `import cv2` → `opencv-python`)
- **Underscores vs hyphens**: Normalize `package-name` ↔ `package_name`
- **Namespace packages**: Handle packages with dots (e.g., `google.cloud.storage`)

### Analysis Accuracy
- **Dynamic imports**: Limited to static analysis (expected limitation)
- **Conditional imports**: May appear as unused but are actually used conditionally
- **Development dependencies**: Distinguish between runtime and dev-only dependencies

## Design Principles Alignment

### Simplicity First
- **Reuse existing infrastructure**: Graph structure, module patterns, CLI patterns
- **Single responsibility**: Each function has one clear purpose
- **Minimal new abstractions**: Extend existing types rather than creating new ones

### Data Integrity  
- **Single source of truth**: PyProject parser as authoritative source for declared dependencies
- **Fail fast**: Return errors for invalid pyproject.toml rather than silent failures
- **Consistent naming**: Use same normalization logic throughout

### Code Quality
- **Self-documenting**: Clear function names and structured results
- **Error handling**: Comprehensive Result types and error messages
- **Testing**: Unit tests for each component, integration tests for workflows

### YAGNI Compliance
- **Build exactly what's specified**: Three analysis types, text output format
- **Structure for extension**: Formatter trait for future output formats
- **No speculative features**: Focus on current requirements

## Future Extensibility

This implementation provides foundation for:
- **JSON/CSV output formats**: Formatter trait ready for extension
- **Dependency vulnerability scanning**: Package list available for security tools
- **License compliance**: Package names available for license checking
- **Update recommendations**: Usage frequency guides update priority