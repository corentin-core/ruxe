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

## Closure captures and mutability

`move` closures take ownership of their captures, but **mutability of the source
binding is preserved** — it doesn't get auto-promoted. If the closure body calls a
captured `FnMut` or mutates a captured value, the source binding must be declared
`mut`.

```rust
// BAD - compile error: cannot borrow `self.callback` as mutable
fn wrap(
    self: Box<Self>,
    next: Box<dyn FnMut(&S, E) -> R>,
) -> Box<dyn FnMut(&S, E) -> R> {
    Box::new(move |state, event| {
        (self.callback)(state);   // needs &mut self.callback
        next(state, event)         // needs &mut next
    })
}

// GOOD - `mut` on bindings enables mutable capture
fn wrap(
    mut self: Box<Self>,
    mut next: Box<dyn FnMut(&S, E) -> R>,
) -> Box<dyn FnMut(&S, E) -> R> {
    Box::new(move |state, event| {
        (self.callback)(state);
        next(state, event)
    })
}
```

The `mut` on a function parameter is a **binding modifier**, not part of the
signature contract — invisible to callers. Keep it when the closure body needs
mutable access to a captured `FnMut`/`FnOnce` or mutates a captured field.

Rule of thumb:

| Capture kind | Called from move closure | `mut` on source binding? |
| ------------ | ------------------------ | ------------------------ |
| `Fn`         | Yes                      | No                       |
| `FnMut`      | Yes                      | **Yes**                  |
| `FnOnce`     | Yes (once)               | No (binding is consumed) |

When reviewing a move closure, ask: *"does the body call a captured `FnMut`, or
write to a captured field?"* If yes, every relevant source binding needs `mut`.

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

## Code review — diagnose before prescribing

When reviewing code, identify the **smallest fix** that addresses the actual problem
before proposing structural changes.

A symptom (verbose paths, repeated boilerplate, awkward call sites) often has multiple
possible fixes ordered by cost:

1. **Local syntactic fix** — `use` import, type alias, `cargo fmt`
2. **In-place rename** — better identifier, no structure change
3. **Local refactor** — extract helper, group related items
4. **Structural refactor** — split modules, change visibility, reorganize file layout

Start at the top of the list and stop at the first fix that resolves the symptom.
Proposing a structural refactor when a `use` statement suffices is **noise** — it
overrides the author's design choices for no real benefit.

Concrete example: long fully-qualified paths inside a nested module (e.g.
`crate::store::tests::fixtures::SimpleState` everywhere) is a **`use` problem**, not
a structural problem. The fix is `use super::super::fixtures::*;`, not flattening
the module hierarchy.

When in doubt, ask the author: *"is the encapsulation intentional, or just an
artifact of the path verbosity?"* — the answer dictates whether to fix locally
or restructure.
