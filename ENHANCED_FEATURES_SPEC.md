# Enhanced Dependency Mapper CLI Specification

## Executive Summary

This document specifies the enhanced features for the Python Dependency Mapper CLI tool, transforming it from a basic analyzer into a comprehensive dependency management toolkit for both human engineers and AI coding assistants (agents). The tool provides actionable insights for code maintenance, refactoring, security auditing, and agentic workflow optimization.

## Current Foundation

The existing tool provides:
- Static Python import analysis using `rustpython-parser`
- Dependency graph construction with three relationship types: `Imports`, `Contains`, `IncludedIn`
- Basic graph traversal and display functionality
- Support for both internal and external module dependencies

## Enhanced Feature Categories

### 1. Core Analysis Commands (Human-Oriented)

#### 1.1 Impact Analysis
**Command**: `dep-mapper impact MODULE_NAME`
**Purpose**: Identify all modules that depend on the specified module (blast radius analysis)
**Use Case**: Before refactoring a module, understand which other modules will be affected
**Output**: List of dependent modules with dependency types
**Example**:
```bash
dep-mapper impact src.payments.processor
# Output:
# Modules depending on 'src.payments.processor':
# - src.api.billing (Imports)
# - src.services.subscription (Imports)
# - tests.test_payments (Imports)
# Total: 3 modules affected
```

#### 1.2 Module Dependencies
**Command**: `dep-mapper dependencies MODULE_NAME`
**Purpose**: Show all dependencies of a specific module
**Use Case**: Understanding what a module needs to function, useful for module extraction or testing
**Output**: List of dependencies with types and origins (internal/external)
**Example**:
```bash
dep-mapper dependencies src.payments.processor
# Output:
# Dependencies of 'src.payments.processor':
# - stripe (External - Imports)
# - src.models.payment (Internal - Imports)
# - src.utils.validation (Internal - Imports)
# Total: 3 dependencies (1 external, 2 internal)
```

#### 1.3 Circular Dependency Detection
**Command**: `dep-mapper cycles`
**Purpose**: Detect and report circular dependencies in the codebase
**Use Case**: Identify architectural problems that make code hard to test and maintain
**Output**: List of circular dependency chains with suggested fixes
**Example**:
```bash
dep-mapper cycles
# Output:
# Circular dependencies found:
# Cycle 1: src.models.user → src.services.auth → src.models.user
# Cycle 2: src.api.orders → src.models.order → src.api.orders
# Total: 2 cycles detected
```

#### 1.4 Dead Code Detection
**Command**: `dep-mapper orphans`
**Purpose**: Find modules with no dependents (potential dead code)
**Use Case**: Identify modules that can be safely removed during cleanup
**Output**: List of modules with no incoming dependencies
**Example**:
```bash
dep-mapper orphans
# Output:
# Orphaned modules (no dependents):
# - src.legacy.old_processor (0 dependents)
# - src.utils.deprecated_helpers (0 dependents)
# Note: Entry points and test modules are excluded from this analysis
```

#### 1.5 External Dependencies Audit
**Command**: `dep-mapper external`
**Purpose**: List all external dependencies across the codebase
**Use Case**: Security auditing, license compliance, dependency management
**Output**: List of external packages with usage statistics
**Example**:
```bash
dep-mapper external
# Output:
# External dependencies:
# - numpy (used by 15 modules)
# - requests (used by 8 modules)
# - stripe (used by 3 modules)
# - pytest (used by 25 modules)
# Total: 4 external packages, 51 total usages
```

#### 1.6 Codebase Metrics
**Command**: `dep-mapper metrics`
**Purpose**: Display overall codebase health indicators
**Use Case**: Architecture assessment, technical debt measurement
**Output**: Key metrics about dependency structure
**Example**:
```bash
dep-mapper metrics
# Output:
# Dependency Metrics:
# - Total modules: 156
# - Total dependencies: 342
# - Circular dependencies: 2
# - Orphaned modules: 3
# - Average dependencies per module: 2.19
# - Most connected module: src.models.user (23 dependents)
# - Deepest dependency chain: 8 levels
```

### 2. Agent Integration Commands (AI-Oriented)

#### 2.1 Smart Context Selection
**Command**: `dep-mapper agent-context MODULE_NAME`
**Purpose**: Output a curated list of files an agent should read when working on a specific module
**Use Case**: Help AI agents include relevant context without token waste
**Output**: Prioritized file list with relevance scores
**Example**:
```bash
dep-mapper agent-context src.payments.processor --format json
# Output:
# {
#   "primary_file": "src/payments/processor.py",
#   "dependencies": [
#     {"file": "src/models/payment.py", "relevance": "high", "reason": "direct import"},
#     {"file": "src/utils/validation.py", "relevance": "medium", "reason": "utility import"}
#   ],
#   "dependents": [
#     {"file": "src/api/billing.py", "relevance": "high", "reason": "imports processor"},
#     {"file": "tests/test_payments.py", "relevance": "low", "reason": "test file"}
#   ],
#   "suggested_token_budget": 1200
# }
```

#### 2.2 Context Suggestions
**Command**: `dep-mapper suggest-context PATH`
**Purpose**: Recommend minimal context for agents working in a directory
**Use Case**: Optimize agent performance when working on features spanning multiple files
**Output**: Smart file selection based on dependency relationships
**Example**:
```bash
dep-mapper suggest-context src/api/ --max-files 10
# Output:
# Suggested context for 'src/api/':
# Essential files (must include):
# 1. src/api/base.py (imported by 8 other API modules)
# 2. src/models/user.py (used by 6 API endpoints)
# 3. src/utils/auth.py (authentication dependency)
#
# Optional files (if token budget allows):
# 4. src/api/orders.py (high coupling with other API modules)
# 5. src/api/payments.py (moderate coupling)
```

#### 2.3 Change Risk Assessment
**Command**: `dep-mapper risk-assess MODULE_NAME`
**Purpose**: Pre-change risk analysis for agents
**Use Case**: Help agents understand the complexity and risk of modifying a module
**Output**: Risk metrics and safety recommendations
**Example**:
```bash
dep-mapper risk-assess src.models.user
# Output:
# Risk Assessment for 'src.models.user':
# - Risk Level: HIGH
# - Dependents: 23 modules
# - Coupling Score: 8.5/10
# - Recommendations:
#   * Consider interface-only changes
#   * Run full test suite after modifications
#   * Review all dependent modules for breaking changes
#   * Consider deprecation strategy for major changes
```

#### 2.4 Dependency Validation
**Command**: `dep-mapper validate-change FROM_MODULE TO_MODULE`
**Purpose**: Check if adding a dependency would create problems
**Use Case**: Prevent agents from introducing circular dependencies
**Output**: Validation result with warnings
**Example**:
```bash
dep-mapper validate-change src.api.orders src.models.order
# Output:
# Validation Result: SAFE
# - No circular dependency would be created
# - Dependency follows architectural patterns
# - Similar pattern exists in src.api.users → src.models.user

dep-mapper validate-change src.models.user src.api.auth
# Output:
# Validation Result: WARNING
# - Would create circular dependency: models.user → api.auth → models.user
# - Violates layered architecture (models should not depend on API layer)
# - Suggested alternative: Move shared logic to src.services.auth
```

#### 2.5 Parallel Work Identification
**Command**: `dep-mapper suggest-parallel-work`
**Purpose**: Identify independent modules for parallel agent work
**Use Case**: Enable multiple agents to work simultaneously without conflicts
**Output**: Groups of modules that can be safely modified in parallel
**Example**:
```bash
dep-mapper suggest-parallel-work
# Output:
# Independent Work Groups:
# Group 1: Payment Processing
# - src.payments.processor
# - src.payments.gateway
# - src.payments.validation
#
# Group 2: User Management
# - src.auth.login
# - src.auth.registration
# - src.auth.password_reset
#
# Group 3: API Documentation
# - src.api.docs
# - src.api.openapi
#
# Note: Groups are independent - agents can work on different groups simultaneously
```

### 3. Output Formats

#### 3.1 JSON Format
**Flag**: `--format json`
**Purpose**: Machine-readable output for CI/CD integration and agent consumption
**Use Case**: Automated analysis, tool integration, programmatic processing
**Structure**:
```json
{
  "command": "impact",
  "target_module": "src.payments.processor",
  "timestamp": "2024-01-15T10:30:00Z",
  "results": {
    "dependents": [
      {
        "module": "src.api.billing",
        "dependency_type": "Imports",
        "origin": "Internal",
        "file_path": "src/api/billing.py"
      }
    ],
    "summary": {
      "total_dependents": 3,
      "risk_level": "medium"
    }
  }
}
```

#### 3.2 DOT Format (Graphviz)
**Flag**: `--format dot`
**Purpose**: Visual dependency graphs
**Use Case**: Architecture documentation, presentations, visual analysis
**Example**:
```bash
dep-mapper impact src.payments.processor --format dot | dot -Tpng > dependency_graph.png
```

#### 3.3 CSV Format
**Flag**: `--format csv`
**Purpose**: Spreadsheet-compatible format for analysis
**Use Case**: Dependency tracking, reporting, data analysis
**Structure**:
```csv
source_module,target_module,dependency_type,origin,file_path
src.api.billing,src.payments.processor,Imports,Internal,src/api/billing.py
src.services.subscription,src.payments.processor,Imports,Internal,src/services/subscription.py
```

#### 3.4 Agent-Optimized Markdown
**Flag**: `--format agent-md`
**Purpose**: Markdown optimized for Claude Code and similar agents
**Use Case**: Direct agent consumption with proper context structuring
**Example**:
```markdown
# Dependency Analysis: src.payments.processor

## Summary
This module is imported by 3 other modules and has medium refactoring risk.

## Direct Dependencies (what this module imports)
- `stripe` (external package) - Payment processing
- `src.models.payment` - Data models
- `src.utils.validation` - Input validation

## Dependents (modules that import this)
- `src.api.billing` - Billing API endpoints
- `src.services.subscription` - Subscription management
- `tests.test_payments` - Unit tests

## Refactoring Recommendations
- Changes to public interface will affect 3 modules
- Consider backward compatibility for interface changes
- Run payment integration tests after modifications
```

#### 3.5 Cursor Context Format
**Flag**: `--format cursor-context`
**Purpose**: File list format optimized for Cursor's context inclusion
**Use Case**: Direct integration with Cursor IDE
**Output**:
```
# Cursor Context for src.payments.processor
@src/payments/processor.py
@src/models/payment.py
@src/utils/validation.py
@src/api/billing.py
@src/services/subscription.py
```

### 4. Filtering and Selection Options

#### 4.1 Pattern Filtering
**Flag**: `--filter PATTERN`
**Purpose**: Include only modules matching a specific pattern
**Use Case**: Focus analysis on specific parts of the codebase
**Examples**:
- `--filter "src.api.*"` - Only API modules
- `--filter "*.test*"` - Only test modules
- `--filter "src.payments.*"` - Only payment-related modules

#### 4.2 Exclusion Filters
**Flag**: `--exclude-test`
**Purpose**: Exclude test modules from analysis
**Use Case**: Focus on production code dependencies

**Flag**: `--exclude-external`
**Purpose**: Exclude external package dependencies
**Use Case**: Focus on internal architecture

#### 4.3 Depth Limiting
**Flag**: `--max-depth N`
**Purpose**: Limit dependency traversal depth
**Use Case**: Avoid overwhelming output in highly connected codebases

#### 4.4 Output Limiting
**Flag**: `--limit N`
**Purpose**: Limit number of results returned
**Use Case**: Get most important results first

### 5. Integration Patterns

#### 5.1 CI/CD Integration
**Use Case**: Automated dependency validation in pull requests
**Example Workflow**:
```bash
# Check for new circular dependencies
dep-mapper cycles --format json --exit-code-on-issues

# Validate that changes don't increase coupling beyond threshold
dep-mapper metrics --format json | jq '.average_dependencies_per_module < 3'

# Generate dependency report for code review
dep-mapper impact $CHANGED_MODULE --format agent-md > dependency_impact.md
```

#### 5.2 IDE Integration
**Use Case**: Real-time dependency analysis in development environments
**Example**:
```bash
# Generate context for current file
dep-mapper agent-context $(current_file) --format cursor-context

# Quick impact check before refactoring
dep-mapper risk-assess $(current_module) --format brief
```

#### 5.3 Agent Orchestration
**Use Case**: Multi-agent workflow coordination
**Example**:
```bash
# Assign work to different agents
dep-mapper suggest-parallel-work --format json > work_assignments.json

# Validate agent's proposed changes
dep-mapper validate-change $AGENT_MODULE $PROPOSED_DEPENDENCY
```

### 6. Implementation Requirements

#### 6.1 Command Structure
- Use `clap` subcommands for clean CLI interface
- Maintain backwards compatibility with existing `analyze` command
- Support global flags that apply to all subcommands

#### 6.2 Graph Algorithms
- **Cycle Detection**: Implement DFS-based cycle detection using `petgraph`
- **Impact Analysis**: Implement reverse dependency traversal
- **Risk Scoring**: Develop coupling metrics based on dependency counts and types
- **Context Ranking**: Algorithm to prioritize files based on dependency relationships

#### 6.3 Output Formatting
- Modular formatter system supporting multiple output types
- Template-based formatting for extensibility
- Streaming output for large dependency graphs

#### 6.4 Performance Requirements
- Handle codebases up to 100k lines (target size)
- Sub-second response time for single module queries
- Memory efficient graph representation
- Caching for repeated analyses

#### 6.5 Error Handling
- Graceful handling of missing modules
- Clear error messages for invalid patterns
- Partial results when some files cannot be parsed

### 7. Example Workflows

#### 7.1 Refactoring Workflow (Human)
```bash
# 1. Understand what will be affected
dep-mapper impact src.legacy.old_system

# 2. Check for architectural issues
dep-mapper cycles

# 3. Plan the refactoring
dep-mapper dependencies src.legacy.old_system
dep-mapper orphans

# 4. Validate after changes
dep-mapper cycles
dep-mapper metrics
```

#### 7.2 Agent Code Review Workflow
```bash
# 1. Agent gets context for changed file
dep-mapper agent-context $CHANGED_FILE --format json

# 2. Agent validates proposed changes
dep-mapper validate-change $MODULE $NEW_DEPENDENCY

# 3. Agent assesses impact
dep-mapper risk-assess $CHANGED_MODULE

# 4. Agent generates review comments with dependency context
dep-mapper impact $CHANGED_MODULE --format agent-md
```

#### 7.3 Security Audit Workflow
```bash
# 1. List all external dependencies
dep-mapper external --format csv > external_deps.csv

# 2. Find modules using specific packages
dep-mapper dependencies --filter "*requests*" --format json

# 3. Assess impact of vulnerable dependency
dep-mapper impact requests --format json
```

### 8. Future Extensibility

#### 8.1 Plugin System
- Support for custom output formatters
- Custom filtering logic
- Integration with external tools

#### 8.2 Language Support
- Framework for adding support for other languages
- Configurable import statement patterns
- Language-specific architectural patterns

#### 8.3 Advanced Analysis
- Semantic dependency analysis (not just syntactic)
- Runtime dependency tracking integration
- Performance impact analysis based on dependency loading

## Conclusion

This specification transforms the dependency mapper from a basic analysis tool into a comprehensive system that serves both human engineers and AI agents. The focus on actionable insights, multiple output formats, and workflow integration makes it valuable for daily development tasks, code quality maintenance, and agentic automation.

The implementation should prioritize the core analysis commands first, followed by agent integration features, with output formats and filtering options developed in parallel to support both use cases.