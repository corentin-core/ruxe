# ruxe

[Redux](https://redux.js.org/)-inspired state management library for Rust with
compile-time safe parallel reducers on isolated state slices.

## Why ruxe?

Existing Rust Redux implementations (redux-rs, rust_redux) lack key features:
RootReducer, SliceReducer, and parallel execution. ruxe fills that gap by leveraging
Rust's ownership model — not as a constraint, but as a feature — to guarantee
data-race-free parallel reducers at compile time.

## Design

```
Store<S, R>            Generic state container
Reducer<S>             Trait with associated Event type — operates on full state
SliceReducer           Trait with associated Slice + Event — operates on a state slice
HasSlice<T>            Bridges a slice to the global state (user-implemented)
ReducerOutput<S, E>    Return type: new state + optional side events
RootReducer            Combines slice reducers (planned)
Middleware             Pre/post dispatch hooks (planned)
ParallelRootReducer    Rayon-based parallel execution (planned)
```

### Core API

```rust
pub trait Reducer<S> {
    type Event;
    fn reduce(&self, state: &S, event: &Self::Event) -> ReducerOutput<S, Self::Event>;
}

pub trait SliceReducer {
    type Event;
    type Slice;
    fn reduce(
        &self,
        slice: &Self::Slice,
        event: &Self::Event,
    ) -> ReducerOutput<Self::Slice, Self::Event>;
}

pub trait HasSlice<T> {
    fn slice(&self) -> &T;
    fn set_slice(self, slice: T) -> Self;
}

pub struct Store<S, R> { /* ... */ }

impl<S, R: Reducer<S>> Store<S, R> {
    pub fn new(state: S, reducer: R) -> Self;
    pub fn dispatch(&mut self, event: R::Event);
    pub fn state(&self) -> &S;
}
```

### Events, not Actions

Redux uses the term "action" for messages dispatched to the store. ruxe uses **event**
instead — a better fit for embedded/IoT contexts where state changes are often triggered
by external signals (sensor readings, price updates, timer ticks), not user interactions.

## Roadmap

| Phase   | Feature                            | Status  |
|---------|------------------------------------|---------|
| 1 — MVP | [Store, Event, Reducer][i2]        | done    |
| 1       | [ReducerOutput][i3]                | done    |
| 1       | [SliceReducer][i4]                 | done    |
| 1       | [Sequential RootReducer][i5]       | planned |
| 1       | [Middleware][i6]                    | planned |
| 1       | [Documentation & EMS example][i7]  | planned |
| 2       | [Parallel RootReducer (rayon)][i8] | planned |
| 2       | [Benchmarks][i9]                   | planned |

[i2]: https://github.com/corentin-core/ruxe/issues/2
[i3]: https://github.com/corentin-core/ruxe/issues/3
[i4]: https://github.com/corentin-core/ruxe/issues/4
[i5]: https://github.com/corentin-core/ruxe/issues/5
[i6]: https://github.com/corentin-core/ruxe/issues/6
[i7]: https://github.com/corentin-core/ruxe/issues/7
[i8]: https://github.com/corentin-core/ruxe/issues/8
[i9]: https://github.com/corentin-core/ruxe/issues/9

See [the project epic](https://github.com/corentin-core/ruxe/issues/1) for the full
design and task breakdown.

> **Note on parallel reducers**: classic Redux is strictly sequential — each reducer sees
> the previous one's changes, making state transitions predictable and easy to debug.
> Parallel reducers trade that guarantee for performance by giving each reducer a frozen
> snapshot of the state before dispatch. This is arguably an anti-pattern in the Redux
> sense, but it's an interesting space to explore in performance-sensitive or embedded
> contexts where state slices are truly independent. ruxe treats it as an opt-in
> experiment, not a default.

## A learning project

ruxe is built as a Rust learning project. The code is written by hand — Claude Code is
configured in **learning mode**: it reviews, challenges, and explains, but does not write
implementation code.

The Claude configuration showcasing this workflow is tracked in the repo:

- [`CLAUDE.md`](CLAUDE.md) — project instructions and learning workflow
- [`.claude/rules/learning-mode.md`](.claude/rules/learning-mode.md) — behavioral
  constraints (what Claude does and doesn't do)
- [`.claude/skills/validate-design/`](.claude/skills/validate-design/) — design
  validation skill

## License

MIT
