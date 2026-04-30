---
paths:
  - "**/*.rs"
---

# Rust Code Quality

These patterns prevent common review comments. Follow them strictly.

## Doc comments - Required on public items

All public functions, types, traits, and modules must have `///` doc comments:

```rust
// BAD
pub fn dispatch(&mut self, event: Event) -> &State {
    // ...
}

// GOOD
/// Dispatches an event through the middleware chain and reducers.
///
/// Returns a reference to the updated state.
pub fn dispatch(&mut self, event: Event) -> &State {
    // ...
}
```

## Doc comments - What NOT to write

A doc comment should **complement** the code, not paraphrase it. The reader will
read the implementation if they need the algorithmic detail.

**Avoid:**

- **Step-by-step algorithm descriptions** — if the doc reads like a numbered list of
  what the function does line by line, delete it. The code already says that.
- **Visual diagrams that mirror the code structure** — a sequence diagram tracing
  every call inside the function is just an algorithm rewrite in another medium.
- **Restating obvious type information** — `Returns a Vec<Event>` adds nothing over
  the signature.
- **Doc on a trait/struct that duplicates doc on a related item** — if the macro
  that generates impls for a trait already documents the composition contract,
  don't repeat that contract on the trait itself.

**Focus on:**

- **The contract** — invariants, ordering guarantees, panics, error conditions
- **Non-obvious bounds and *why* they exist** — `S: Clone` is needed because
  `set_slice` consumes `self`, so we must own the state
- **Edge cases and gotchas** — "returns `None` if the queue is empty, even after
  side events were produced"
- **A concrete usage example** when the API shape is non-obvious (tuple impl,
  builder pattern, complex generics)

When in doubt, ask: *"would this sentence still be true if the implementation
changed completely?"* If yes, it's contract — keep it. If no, it's a paraphrase —
delete it.

## Error handling - Use Result, not panic

Library code should never panic. Use `Result` types for fallible operations:

```rust
// BAD
pub fn dispatch(&mut self, event: Event) -> &State {
    if self.depth > MAX_DEPTH {
        panic!("Max re-dispatch depth exceeded");
    }
    // ...
}

// GOOD
pub fn dispatch(&mut self, event: Event) -> Result<&State, DispatchError> {
    if self.depth > MAX_DEPTH {
        return Err(DispatchError::MaxDepthExceeded);
    }
    // ...
}
```

## Type safety over runtime checks

Use the type system to prevent invalid states at compile time.

## Prefer generics over trait objects

Use static dispatch (generics) by default. Only use dynamic dispatch (`dyn Trait`) when
truly needed (heterogeneous collections, plugin systems).

## Derive common traits

Always derive standard traits when applicable:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ReducerOutput<S> {
    pub state: S,
    pub events: Vec<Event>,
}
```

Derive order convention: `Debug, Clone, Copy, PartialEq, Eq, Hash, Default`

## Visibility - Private by default

Keep items private unless there's a reason to expose them.

## Use `#[must_use]` on important return values

## Avoid premature optimization

Keep code simple first. Use `criterion` benchmarks to justify optimizations.

## No unsafe without justification

`unsafe` blocks require:

1. A `// SAFETY:` comment explaining why it's sound
2. Discussion with the user before adding

## Naming conventions

Follow Rust naming conventions:

| Item        | Convention    | Example              |
| ----------- | ------------- | -------------------- |
| Types       | `PascalCase`  | `RootReducer`        |
| Functions   | `snake_case`  | `dispatch_event`     |
| Constants   | `UPPER_SNAKE` | `MAX_DISPATCH_DEPTH` |
| Modules     | `snake_case`  | `root_reducer`       |
| Traits      | `PascalCase`  | `SliceReducer`       |
| Type params | `PascalCase`  | `S`, `State`, `E`    |
