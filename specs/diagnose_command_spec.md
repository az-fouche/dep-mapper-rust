# Diagnose Command Specification v2

## Overview

The `diagnose` command provides a comprehensive health report of the codebase from a dependency perspective. It aggregates all existing analysis capabilities into a weighted, actionable summary with configurable thresholds and context-aware recommendations.

## Enhanced Metrics

### 1. Module Classification
Classify modules before analysis:
- **Production**: Regular application modules
- **Test**: Test modules (containing 'test' in path)
- **API**: Public interface modules
- **Core**: Business logic (should be stable)
- **Utility**: Shared helpers (expected high fan-in)

### 2. Cohesion & Coupling Metrics
- **Instability (I)**: `Ce / (Ca + Ce)` where Ca = afferent coupling, Ce = efferent coupling
- **Distance from Main Sequence (D)**: `|A + I - 1|` where A = abstractness
- Ideal: Stable modules should be abstract, unstable modules should be concrete

### 3. Circular Dependency Severity
Classify cycles by impact:
- **Critical**: Production cycles with >3 modules
- **High**: Production cycles with 2-3 modules  
- **Medium**: Cycles involving test modules
- **Low**: Cycles within same package

### 4. Import Pattern Quality
- Flag wildcard imports (`from x import *`)
- Track import depth (e.g., `a.b.c.d.e` = depth 5)
- Identify excessive chaining (depth > 5)

## Health Score Calculation

### Weighted Scoring (0-100)
```python
score = 100.0
for metric, value in metrics:
    weight = config.weights[metric]
    if value > config.thresholds.critical[metric]:
        score -= weight * 30  # Critical penalty
    elif value > config.thresholds.warning[metric]:
        score -= weight * 15  # Warning penalty
return max(0, score)
```

### Health Grades
- **A (90-100)**: Excellent architecture
- **B (80-89)**: Good, minor improvements needed
- **C (70-79)**: Fair, notable issues present
- **D (60-69)**: Poor, significant refactoring needed
- **F (<60)**: Critical, major overhaul required

## Report Structure

```
CODEBASE HEALTH REPORT
======================

📊 HEALTH SCORE: 78/100 (Grade: C)
   Trend: ↓ -3 points from last analysis

🎯 TOP PRIORITY ISSUES
----------------------
1. 🔴 CRITICAL: 3 circular dependencies in production
   → auth.manager ↔ models.user ↔ auth.permissions
   → Fix: Extract shared interfaces

2. ⚠️ HIGH: Module 'core.utils' has 47 dependents
   → Fix: Split into focused sub-modules

3. ⚠️ MEDIUM: 5 undeclared dependencies
   → Fix: Add to pyproject.toml

📈 ARCHITECTURE METRICS
-----------------------
Modules: 234 total (186 production, 48 test)
Dependencies: 1,247 imports (avg 5.3 per module)
Depth: Max 8, Avg 3.2

Cohesion & Coupling:
• Average Instability: 0.42
• Distance from Main: 0.28 ⚠️
• High-pressure modules: 12

📦 EXTERNAL DEPENDENCIES
------------------------
• 5 undeclared, 2 unused
• Top: numpy (45 modules), pandas (32)
• Concentration risk: 60% used by <10% of modules

🔄 CIRCULAR DEPENDENCIES
------------------------
[CRITICAL] auth → models → auth (affects 47 modules)
[CRITICAL] core.cache → core.database → core.cache
[HIGH] utils.helpers → utils.validators → utils.helpers

📋 RECOMMENDATIONS
------------------
Immediate:
1. Break auth → models cycle
2. Add undeclared dependencies
3. Split core.utils module

Short-term:
1. Reduce coupling in analytics.calculator
2. Fix layer violations (3 found)
```