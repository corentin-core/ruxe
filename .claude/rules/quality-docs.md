# Quality Documentation

## User-Observable Behaviors Only

**Why**: Quality docs describe what users experience, not implementation details.

**Rule**: ALWAYS write scenarios from the user's perspective.

```markdown
# BAD - implementation details

> **Given** a `ReducerOutput` with an empty events vec **When** `dispatch()` is called
> **Then** no re-dispatch loop occurs

# GOOD - user perspective

> **Given** a reducer that returns no secondary events **When** the user dispatches an
> event **Then** the state is updated in a single pass
```

## Focus on Emergent Behaviors

Quality docs capture behaviors that emerge from the system. Document what might surprise
a user or what requires multiple components working together.

Good candidates:

- Parallel vs sequential semantic differences
- Re-dispatch event ordering
- Middleware chain execution order
- Compile-time error messages when slice isolation is violated

Bad candidates:

- Basic trait method signatures
- Internal data structures
- Standard Rust patterns (Clone, Debug, etc.)