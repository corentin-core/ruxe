# Documentation Guidelines

## Language

**Rule**: ALWAYS write documentation in English.

This ensures consistency across the codebase and makes the project accessible to a wider
audience.

## No File Paths in docs

**Why**: File paths create maintenance burden when refactoring.

**Rule**: NEVER include file paths in documentation unless absolutely necessary. Describe
components by name, not location.

## Describe Responsibilities, Not Method Signatures

**Why**: Method signatures are implementation details that duplicate what's already in
the code and rustdoc.

**Rule**: Describe component responsibilities in natural language bullet points.

```markdown
# BAD - lists methods (couples doc to implementation)

| Method        | Purpose                    |
| ------------- | -------------------------- |
| `dispatch()`  | Send event to reducers     |
| `get_state()` | Returns current state ref  |

# GOOD - describes what the component does

**Store** is the central state container. It:

- Holds the application state
- Routes events through the middleware chain to reducers
- Provides read access to the current state
```

## Prefer Diagrams Over Text

When a diagram clearly conveys the information, don't add redundant text. Use Mermaid
for:

- Architecture overviews (graph TB)
- Trait relationships (classDiagram)
- Event flows (sequenceDiagram)
- State machines (stateDiagram-v2)

## Rustdoc

All public items must have `///` doc comments. Include:

- A one-line summary
- `# Examples` section with a compilable code block when useful
- `# Errors` section for functions returning `Result`
- `# Panics` section if the function can panic

```rust
/// Dispatches an event through the middleware chain and reducers.
///
/// # Errors
///
/// Returns `DispatchError::MaxDepthExceeded` if re-dispatch depth exceeds the limit.
///
/// # Examples
///
/// ```
/// let mut store = Store::new(AppState::default(), root_reducer);
/// store.dispatch(Event::Tick)?;
/// assert_eq!(store.get_state().counter, 1);
/// ```
pub fn dispatch(&mut self, event: Event) -> Result<&S, DispatchError> {
```