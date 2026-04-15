# Architecture

## Reducer traits

Ruxe exposes two reducer traits with different scopes:

- `Reducer<S>` — operates on the full state tree
- `SliceReducer` — operates on an isolated slice of the state

Both produce a `ReducerOutput` containing the new state and optional side events
to re-dispatch.

`HasSlice<T>` bridges the global state to a slice: the user implements it on their
state struct to expose each slice for reading (`slice`) and replacement
(`set_slice`).

```mermaid
classDiagram
    class Reducer~S~ {
        <<trait>>
        +Event
        +reduce(state: &S, event: &Event) ReducerOutput~S, Event~
    }

    class SliceReducer {
        <<trait>>
        +Event
        +Slice
        +reduce(slice: &Slice, event: &Event) ReducerOutput~Slice, Event~
    }

    class HasSlice~T~ {
        <<trait>>
        +slice() &T
        +set_slice(slice: T) Self
    }

    class ReducerOutput~S, E~ {
        +state: S
        +side_events: Option~Vec~E~~
    }

    Reducer ..> ReducerOutput : returns
    SliceReducer ..> ReducerOutput : returns
    HasSlice ..> SliceReducer : bridges global state to slice
```

## Isolation by construction

Slice reducers cannot access other slices. The `SliceReducer::reduce` method
only receives `&Self::Slice` — no other slice is in scope, so access is
structurally impossible rather than checked at runtime.

This property is what makes slice reducers candidates for parallel execution
(planned).
