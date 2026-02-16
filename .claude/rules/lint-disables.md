# Lint Disables

## Never Disable Lint Silently

**Why**: Disabling warnings hides problems. Each disable is a conscious tradeoff that
the user must validate.

**Rule**: ALWAYS ask the user before adding any `#[allow(...)]`, `#[cfg_attr(...)]`, or
clippy suppression.

When you encounter a clippy warning:

1. **Understand** why it triggers
2. **Fix** the underlying issue if possible
3. **If an allow is truly needed**, explain the tradeoff and ask for approval
4. **Scope narrowly** - prefer `#[allow(...)]` on the specific item, not module-wide

```rust
// BAD - silently suppressing
#![allow(clippy::all)]

// BAD - too broad
#[allow(unused)]
fn some_function() { ... }

// GOOD - fix the code or ask first
// "Clippy suggests using `if let` but the match is more readable here because X.
//  Should I add #[allow(clippy::single_match)] on this function?"
```