pub fn print_agent_documentation() {
    print!(
        r#"PYTHON DEPENDENCY ANALYSIS COMMANDS

Health Assessment:
  diagnose             → Comprehensive codebase health report with scoring
                         Example: pydep-mapper diagnose
                         Output: Health score (0-100), metrics summary, issues found
                         Use: Get overall architecture quality assessment

Change Planning:
  changeset MODULE     → Analyze change impact and dependencies for safe refactoring
                         Example: pydep-mapper changeset auth.models --scope both
                         Output: Affected modules, dependencies, risk levels, test order
                         Use: Plan changes, assess blast radius, optimize testing

Exploration Commands:
  pressure             → Find critical modules by dependent count
                         Example: pydep-mapper pressure
                         Output: Ranked list with counts (utils: 45 dependents)
                         Tip: use with |head or |tail top capture top/bottom

  instability          → Find unstable modules by coupling metrics
                         Example: pydep-mapper instability
                         Output: Ranked list with scores (api.handlers: 0.85)
                         Tip: use with |head or |tail top capture top/bottom

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

Changeset Scope Options:
  --scope affected     → Show only what breaks if module changes
  --scope dependencies → Show only what module depends on
  --scope both         → Show both (default)

Output Format: Hierarchical text with submodule counts, excludes test modules
"#
    );
}
