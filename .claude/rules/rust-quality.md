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

Use the type system to prevent invalid states at compile time:

```rust
// BAD - runtime check
fn add_reducer(&mut self, reducer: Box<dyn Reducer>) {
    if self.reducers.len() >= MAX_REDUCERS {
        panic!("too many reducers");
    }
    self.reducers.push(reducer);
}

// GOOD - type system enforces constraints
struct RootReducer<R1, R2, R3> {
    r1: R1,
    r2: R2,
    r3: R3,
}
```

## Prefer generics over trait objects

Use static dispatch (generics) by default. Only use dynamic dispatch (`dyn Trait`) when
truly needed (heterogeneous collections, plugin systems):

```rust
// BAD - unnecessary dynamic dispatch
fn process(reducer: &dyn Reducer<AppState>) -> AppState { ... }

// GOOD - zero-cost static dispatch
fn process<R: Reducer<AppState>>(reducer: &R) -> AppState { ... }
```

## Derive common traits

Always derive standard traits when applicable:

```rust
// GOOD
#[derive(Debug, Clone, PartialEq)]
pub struct ReducerOutput<S> {
    pub state: S,
    pub events: Vec<Event>,
}
```

Derive order convention: `Debug, Clone, Copy, PartialEq, Eq, Hash, Default`

## Visibility - Private by default

Keep items private unless there's a reason to expose them:

```rust
// BAD - everything public
pub struct Store<S> {
    pub state: S,
    pub middlewares: Vec<Box<dyn Middleware<S>>>,
}

// GOOD - minimal public surface
pub struct Store<S> {
    state: S,
    middlewares: Vec<Box<dyn Middleware<S>>>,
}

impl<S> Store<S> {
    pub fn get_state(&self) -> &S { &self.state }
}
```

## Use `#[must_use]` on important return values

```rust
#[must_use]
pub fn dispatch(&mut self, event: Event) -> Result<&State, DispatchError> {
    // ...
}
```

## Avoid premature optimization

Keep code simple first. Use `criterion` benchmarks to justify optimizations:

```rust
// BAD - premature optimization
let mut state = unsafe { std::mem::transmute(...) };

// GOOD - simple, correct, benchmark later
let state = reducer.reduce(state, &event);
```

## No unsafe without justification

`unsafe` blocks require:

1. A `// SAFETY:` comment explaining why it's sound
2. Discussion with the user before adding

```rust
// GOOD - justified unsafe
// SAFETY: We guarantee non-overlapping mutable borrows because each reducer
// operates on a distinct field, enforced by the type system at compile time.
unsafe {
    let slice_a = &mut (*ptr).battery;
    let slice_b = &mut (*ptr).grid;
}
```

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
