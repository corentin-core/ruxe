# ruxe

[Redux](https://redux.js.org/)-inspired state management library for Rust with compile-time safe parallel reducers on isolated state slices.

## Why ruxe?

Existing Rust Redux implementations (redux-rs, rust_redux) lack key features: RootReducer, SliceReducer, and parallel execution. ruxe fills that gap by leveraging Rust's ownership model — not as a constraint, but as a feature — to guarantee data-race-free parallel reducers at compile time.

## Quick start

```rust
use ruxe::{Reducer, ReducerOutput, Store};

// 1. Define your state. It must be `Clone`.
#[derive(Clone)]
struct Counter {
    value: i32,
}

// 2. Define events as a typed enum — no stringly-typed actions.
enum Event {
    Increment,
    Decrement,
}

// 3. A reducer is a pure `(state, event) -> new state` transformation.
struct CounterReducer;

impl Reducer<Counter> for CounterReducer {
    type Event = Event;

    fn reduce(&self, state: &Counter, event: &Event) -> ReducerOutput<Counter, Event> {
        let value = match event {
            Event::Increment => state.value + 1,
            Event::Decrement => state.value - 1,
        };
        ReducerOutput {
            state: Counter { value },
            side_events: None,
        }
    }
}

fn main() {
    // No middlewares; cap side-event re-dispatch depth at 8.
    let mut store = Store::new(Counter { value: 0 }, CounterReducer, vec![], 8);

    store.dispatch(Event::Increment).unwrap();
    store.dispatch(Event::Increment).unwrap();
    store.dispatch(Event::Decrement).unwrap();

    assert_eq!(store.state().value, 1);
}
```

For a realistic multi-slice setup with middlewares, side events, and a side-by-side sequential-vs-parallel comparison, see [`examples/ems.rs`](examples/ems.rs) — a mini Energy Management System composing three independent state slices (solar, battery, grid meter).

## Root reducers — sequential and parallel

When your state is composed of multiple independent slices, define one [`SliceReducer`] per slice, then combine them with one of two root reducers:

```rust
use ruxe::{SequentialRootReducer, ParallelRootReducer};

// Sequential: each reducer sees the previous one's state changes,
// applied in tuple declaration order.
let root = SequentialRootReducer::new((solar_reducer, battery_reducer, meter_reducer));

// Parallel: every reducer reads from the same pre-event state, runs on a
// Rayon worker. Requires the state to implement `StateSlices` declaring
// its slice list — see below.
let root = ParallelRootReducer::new((solar_reducer, battery_reducer, meter_reducer));
```

### Compile-time guarantees of `ParallelRootReducer`

The parallel variant only compiles if the slice-to-reducer mapping is a bijection. Two cases produce a compile error:

| Mistake | Compile error |
| --- | --- |
| Two reducers target the same slice | "ambiguous impl" — the compiler can't resolve which reducer owns the slice |
| A state slice has no matching reducer | "trait bound not satisfied" — no reducer found for the slice |

No runtime check, no data race possible. The compiler refuses to build a parallel root reducer that would race on a shared slice. See [`examples/ems.rs`](examples/ems.rs) for a runnable side-by-side comparison showing a ~3x speedup with three slice reducers each doing ~50ms of work.

### Declaring the slice list

`ParallelRootReducer` requires the state to declare its slice types via [`StateSlices`]. Rust's type system can't enumerate impls of `HasSlice<T>`, so the user lists them explicitly:

```rust
use ruxe::{HasSlice, StateSlices};

impl StateSlices for AppState {
    type Slices = ruxe::HList!(CounterSlice, UserSlice);
}
```

A future `#[derive(StateSlices)]` will generate this from the struct fields.

## Design

### Runtime pipeline

```mermaid
flowchart LR
    User([User code]) -->|dispatch event| Store
    Store -->|"wraps in onion chain"| MW[Middleware chain]
    MW -->|"calls"| R[Reducer]
    R -->|"new state + side events"| Store
```

### Composing slice reducers into a Reducer

```mermaid
flowchart TB
    SR1[SliceReducer A] & SR2[SliceReducer B] & SR3[SliceReducer ...] -->|tuple| Wrapper{{SequentialRootReducer<br/>or<br/>ParallelRootReducer}}
    Wrapper -.implements.-> R[Reducer trait]

    State[State struct] -.must impl HasSlice&lt;T&gt; per slice.-> SR1
    ParRR[ParallelRootReducer only] -.requires.-> SS[StateSlices on State]

    style Wrapper fill:#f9f,stroke:#333,stroke-width:2px
```

Slice reducers compose into a root reducer via either [`SequentialRootReducer<T>`] (applies them in order, threading state through `set_slice`) or [`ParallelRootReducer<L, E, Indices>`] (applies them on Rayon workers, with compile-time disjointness verification). `Next<S, E>` is the dispatch-chain closure each middleware wraps, and `DispatchError` is returned when side-event recursion exceeds the configured depth.

For exact signatures, trait bounds, and runnable examples, see the rustdoc — run `cargo doc --open` (it will be published on docs.rs once ruxe ships to crates.io).

### Events, not Actions

Redux uses the term "action" for messages dispatched to the store. ruxe uses **event** instead — a better fit for embedded/IoT contexts where state changes are often triggered by external signals (sensor readings, price updates, timer ticks), not user interactions.

## Comparison with redux-rs

[redux-rs](https://github.com/redux-rs/redux-rs) is the most established Redux port for Rust. The two libraries make different tradeoffs:

| Aspect                     | redux-rs                                            | ruxe                                                        |
| -------------------------- | --------------------------------------------------- | ----------------------------------------------------------- |
| Message term               | `Action`                                            | `Event`                                                     |
| State structure            | single state; every reducer sees all of it          | isolated slices — a `SliceReducer` can only reach its slice |
| Root composition           | one reducer combines everything by hand             | wrapped in `SequentialRootReducer` / `ParallelRootReducer`  |
| Reducer input              | `state: State` (owned, mutate-and-return, no clone) | `state: &S` (borrowed; reducer returns a fresh state)       |
| Side events in reducer     | none — reducers are pure `state → state`            | reducers may emit side events, re-dispatched by the store   |
| Side effects in middleware | yes — re-dispatch via `inner.dispatch().await`      | yes — return side events that the store queues              |
| Execution model            | async-native, requires Tokio                        | synchronous, runtime-agnostic (async planned)               |
| Concurrency                | lock-free multi-producer dispatch via a worker task | single-owner `&mut self`, parallel reducers via Rayon       |
| Middleware registration    | manual `.wrap(m).await` chain, separate store type  | passed to `Store::new` (empty `vec` to opt out)             |
| Selectors                  | built-in (`select`, memoized state queries)         | none — slices are accessed directly via `HasSlice`          |
| Parallel reducers          | no                                                  | `ParallelRootReducer` (rayon, compile-time disjoint)        |

In short: redux-rs is async-first and ships more batteries (selectors, Tokio integration); ruxe trades those for **compile-time slice isolation**, **side events straight from reducers**, and a **runtime-agnostic synchronous core** that you wrap in whatever execution model you need.

## Roadmap

| Phase   | Feature                            | Status  |
|---------|------------------------------------|---------|
| 1 — MVP | [Store, Event, Reducer][i2]        | done    |
| 1       | [ReducerOutput][i3]                | done    |
| 1       | [SliceReducer][i4]                 | done    |
| 1       | [Sequential RootReducer][i5]       | done    |
| 1       | [Middleware][i6]                   | done    |
| 1       | [Documentation & EMS example][i7]  | done    |
| 2       | [Parallel RootReducer (rayon)][i8] | done    |
| 2       | [Benchmarks][i9]                   | planned |

[i2]: https://github.com/corentin-core/ruxe/issues/2
[i3]: https://github.com/corentin-core/ruxe/issues/3
[i4]: https://github.com/corentin-core/ruxe/issues/4
[i5]: https://github.com/corentin-core/ruxe/issues/5
[i6]: https://github.com/corentin-core/ruxe/issues/6
[i7]: https://github.com/corentin-core/ruxe/issues/7
[i8]: https://github.com/corentin-core/ruxe/issues/8
[i9]: https://github.com/corentin-core/ruxe/issues/9

See [the project epic](https://github.com/corentin-core/ruxe/issues/1) for the full design and task breakdown.

> **Note on parallel reducers**: classic Redux is strictly sequential — each reducer sees the previous one's changes, making state transitions predictable and easy to debug. Parallel reducers trade that guarantee for performance by giving each reducer a frozen snapshot of the state before dispatch. This is arguably an anti-pattern in the Redux sense, but it's an interesting space to explore in performance-sensitive or embedded contexts where state slices are truly independent. ruxe treats it as an opt-in experiment, not a default.

## A learning project

ruxe is built as a Rust learning project. The code is written by hand — Claude Code is configured in **learning mode**: it reviews, challenges, and explains, but does not write implementation code.

The Claude configuration showcasing this workflow is tracked in the repo:

- [`CLAUDE.md`](CLAUDE.md) — project instructions and learning workflow
- [`.claude/rules/learning-mode.md`](.claude/rules/learning-mode.md) — behavioral constraints (what Claude does and doesn't do)
- [`.claude/skills/validate-design/`](.claude/skills/validate-design/) — design validation skill

## License

MIT
