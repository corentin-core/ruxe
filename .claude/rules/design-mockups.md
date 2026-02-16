# Design Mockups

## Trait Designs Require Signatures

**Why**: Text descriptions of APIs are ambiguous. Showing trait signatures makes designs
reviewable before implementation.

**Rule**: ALWAYS include trait/struct signatures when designing features.

### What to Include

1. **Trait signatures** - Show the full trait definition with associated types
2. **Concrete examples** - Show how a user would implement the trait
3. **Interaction patterns** - Show how components compose together

### Example

```markdown
## SliceReducer Design

### Trait signature

\`\`\`rust
pub trait SliceReducer {
    type Slice;

    fn reduce(&self, slice: &Self::Slice, event: &Event) -> SliceOutput<Self::Slice>;
}
\`\`\`

### User implementation

\`\`\`rust
struct BatteryReducer;

impl SliceReducer for BatteryReducer {
    type Slice = BatteryState;

    fn reduce(&self, slice: &BatteryState, event: &Event) -> SliceOutput<BatteryState> {
        match event {
            Event::BatteryUpdated { soc, power_w } => {
                SliceOutput::new(BatteryState { soc: *soc, power_w: *power_w })
            }
            _ => SliceOutput::unchanged(slice.clone()),
        }
    }
}
\`\`\`

### Composition

\`\`\`rust
let root = RootReducer::sequential(battery_reducer, grid_reducer);
let store = Store::new(AppState::default(), root);
\`\`\`
```

### Feature Explanations

Below each design, include:

- **Trade-offs** - Why this approach over alternatives
- **Constraints** - What the type system enforces
- **Open questions** - Anything needing discussion