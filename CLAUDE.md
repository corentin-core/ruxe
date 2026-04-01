# Ruxe

Redux-inspired state management library for Rust with compile-time safe parallel
reducers on isolated state slices.

## Architecture

```
src/
  store.rs          # Store<State> — typed state container
  event.rs          # Event trait / enum
  reducer.rs        # Reducer, SliceReducer traits
  root_reducer.rs   # RootReducer (sequential)
  parallel.rs       # ParallelRootReducer (rayon-based)
  middleware.rs      # Middleware trait and chain
  output.rs         # ReducerOutput, SliceOutput
  lib.rs            # Public API re-exports
tests/              # Integration tests
benches/            # Criterion benchmarks
examples/           # Usage examples (EMS)
```

## Common commands

```bash
# Build
cargo build

# Run tests
cargo test

# Run a specific test
cargo test test_name -- --nocapture

# Run clippy
cargo clippy -- -D warnings

# Format code
cargo fmt

# Run benchmarks
cargo bench

# Generate documentation
cargo doc --open
```

## Code conventions

- Rust 2021 edition
- All public items must have doc comments (`///`)
- Type safety enforced at compile time (no runtime type checks where avoidable)
- `#[must_use]` on functions returning values that shouldn't be ignored
- Prefer owned types in public APIs, references internally
- Always run `cargo test` before committing

See `.claude/rules/rust-quality.md` for detailed patterns and examples.

## Learning Workflow

This is a **learning project**. The user designs and implements. Claude validates and
supports. See `.claude/rules/learning-mode.md` for the full behavioral constraint.

```
User designs  →  Claude validates (/validate-design)
User codes    →  Claude reviews (ad-hoc or /review on PR)
User is stuck →  Claude explains, gives hints
Code is ready →  Claude writes docs (/document), user commits
```

**Key rule**: Claude does NOT write implementation code unless explicitly asked.

## Design principles

- **Leverage the type system** - Compile-time guarantees over runtime checks
- **Zero-cost abstractions** - Traits and generics, not dynamic dispatch (unless needed)
- **No unsafe** unless absolutely necessary and documented

## Important notes

- Project epic: GitHub issue #1
- Parallel reducers use rayon for work-stealing parallelism
- State is generic — users define their own state structs
- Events use Rust enums (exhaustive pattern matching, no stringly-typed actions)

## Git workflow

See `.claude/rules/git-conventions.md` for branch naming, commit messages, worktrees,
and merge workflow.

## PR review workflow

See `.claude/commands/handle-pr-comments.md` for the full PR comment handling workflow
(inline replies, `🤖 Claude:` prefix, commit references).

## Creating GitHub issues

All issues must be created in **English** with the following structure:

### Required elements

1. **Labels** (add with `gh issue edit {id} --add-label "label"`):

   - Priority: `priority:high`, `priority:medium`, `priority:low`
   - Theme: `enhancement`, `bug`, `refactor`, `documentation`, `testing`
   - Phase: `phase:mvp`, `phase:parallel`, `phase:future`

2. **Issue body structure**: Context, Proposed solution, Acceptance criteria, Related
   issues

3. **Link related issues** using keywords: `depends on`, `blocks`, `related to`

## Testing Principles

See `.claude/rules/testing.md` for the full testing strategy.

## Workflow Automation

- **Wait for the user** - Do not invoke skills proactively. The user drives the workflow
  and calls `/validate-design`, `/document`, `/review`, `/commit`, `/create-pr` when
  ready.

## Working Principles

### Code Navigation with MCP Serena

**Prefer MCP Serena tools** for code navigation and exploration when available.

### Coherence with Existing Codebase

When using internal modules, check how they're used in related code.

### Apply Changes Globally

When a fix or pattern is requested, search for the same pattern elsewhere in the code
and fix all occurrences for consistency.

## META - Self-Improvement System

See `.claude/rules/auto-introspection.md` for the self-improvement workflow (triggered
when the user points out mistakes, proposes config updates).