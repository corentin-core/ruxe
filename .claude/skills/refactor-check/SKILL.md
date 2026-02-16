---
name: refactor-check
description:
  Analyze code for refactoring opportunities, API design issues, and test quality
---

# Refactor Check

Analyze code for refactoring opportunities: API design issues, missing type safety, and
test quality problems.

## Persona

You are a **code quality analyst** focused on identifying structural improvements that
make code more maintainable, safe, and idiomatic.

## Arguments

- `$ARGUMENTS`: File path, directory, or module name to analyze. If empty, analyze
  recently modified files.

## Instructions

### Step 1: Identify scope

```bash
# If argument is a file or directory
ls $ARGUMENTS

# If no argument, check recent changes
git diff main --name-only | grep '\.rs$'
```

### Step 2: Check API design

For each public type/trait:

- Is the public surface minimal? (no unnecessary `pub`)
- Are return types appropriate? (owned vs borrowed)
- Are error types expressive enough?
- Is the API consistent with other modules?

### Step 3: Check type safety

- Can any runtime checks be replaced with compile-time guarantees?
- Are there `unwrap()`s that should be `?` or proper error handling?
- Are there `clone()`s that could be avoided with better ownership?

### Step 4: Check test quality

**Tests to flag:**

1. **Tests implementation, not behavior** - Tests internal state
2. **Tests Rust stdlib** - Verifies derive macros work
3. **Missing edge cases** - No error path tests, no boundary tests

### Step 5: Generate summary

```markdown
## Refactor Check Summary

**Scope:** [files analyzed]

### Quick Wins (Low effort, high impact)

- ...

### Recommended Refactors

- ...

### Technical Debt to Track

- ...
```

$ARGUMENTS