---
paths:
  - "**/tests/**/*.rs"
  - "**/*_test.rs"
  - "**/test_*.rs"
---

# Testing Standards

## Two-tier testing strategy

Both types of tests are required:

1. **Unit tests**: `#[cfg(test)] mod tests` inside each source file. Test each component
   in isolation. Fast, precise, easy to debug.
2. **Integration tests**: `tests/` directory. Verify components work together. Exercise
   the complete flow to catch interface mismatches.

Unit tests alone are NOT sufficient — integration tests catch bugs that mocks hide.

## Test actual behavior, not implementation

```rust
// BAD - tests internal state
#[test]
fn test_internal_reducer_count() {
    let root = RootReducer::new(r1, r2);
    assert_eq!(root.reducers.len(), 2); // Testing internals
}

// GOOD - tests observable behavior
#[test]
fn test_root_reducer_applies_all_reducers() {
    let root = RootReducer::new(increment_reducer, double_reducer);
    let state = AppState { counter: 1 };
    let result = root.reduce(&state, &Event::Tick);
    assert_eq!(result.state.counter, 4); // 1 + 1 = 2, 2 * 2 = 4
}
```

## Assert on whole structs, not individual fields

When types derive `PartialEq`, compare entire structs:

```rust
// BAD - misses unexpected changes in other fields
assert_eq!(output.state.counter, 5);

// GOOD - catches any unexpected change
let expected = AppState { counter: 5, flag: true };
assert_eq!(output.state, expected);
```

## Use descriptive test names

Test names should describe the scenario, not the implementation:

```rust
// BAD
#[test]
fn test_dispatch() { ... }

// GOOD
#[test]
fn dispatch_increments_counter_on_increment_event() { ... }

#[test]
fn dispatch_returns_error_on_max_redispatch_depth() { ... }
```

## Use test helpers and builders

For complex state setup, create builder helpers in test modules:

```rust
#[cfg(test)]
mod tests {
    fn make_state(counter: i32) -> AppState {
        AppState { counter, ..Default::default() }
    }

    #[test]
    fn dispatch_with_initial_state() {
        let state = make_state(10);
        // ...
    }
}
```

## Test edge cases

For each feature, test:

- Happy path (normal operation)
- Boundary conditions (empty collections, zero values)
- Error paths (invalid input, max depth)
- Ordering (sequential reducer order matters)

## No tautological tests

Don't test Rust language guarantees:

```rust
// BAD - tests that Vec::push works
#[test]
fn test_vec_push() {
    let mut v = vec![1];
    v.push(2);
    assert_eq!(v.len(), 2); // Tests std, not your code
}

// BAD - tests that Clone works
#[test]
fn test_state_clone() {
    let s = AppState::default();
    let s2 = s.clone();
    assert_eq!(s, s2); // Tests #[derive(Clone)]
}
```