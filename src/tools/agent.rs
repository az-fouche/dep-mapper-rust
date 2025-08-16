pub fn print_agent_documentation() {
    print!(
        r#"PYTHON DEPENDENCY ANALYSIS COMMANDS

Exploration Commands:
  pressure             → Find critical modules by dependent count
                         Example: pydep-mapper pressure
                         Output: Ranked list with counts (utils: 45 dependents)

  external             → Audit third-party package usage with frequency
                         Example: pydep-mapper external  
                         Output: requests (23 imports), pandas (12 imports)

  cycles                → Detect circular dependencies (architectural issues)
                         Example: pydep-mapper cycles
                         Output: a.models → b.utils → a.models

Target Analysis Commands:
  impact MODULE        → Find blast radius - what breaks if MODULE changes
                         Example: pydep-mapper impact auth.models
                         Output: All modules importing auth.models (transitively)

  dependencies MODULE  → Find what MODULE imports (understand requirements)
                         Example: pydep-mapper dependencies api.views
                         Output: All imports used by api.views (internal + external)

Global Options:
  --root DIR           → Analyze specific directory (default: current dir)
                         Example: pydep-mapper --root /path/to/project pressure

Output Format: Hierarchical text with submodule counts, excludes test modules
"#
    );
}