//! Parallel root reducer: applies a tuple of [`SliceReducer`]s in parallel
//! via Rayon, with compile-time guarantees that each state slice has exactly
//! one matching reducer. See [`ParallelRootReducer`] for usage.

use crate::hlist::{HCons, HNil, IntoHList};
use crate::indices::{Here, There};
use crate::reducer::{Reducer, ReducerOutput, SliceReducer};
use crate::state::HasSlice;
use crate::state::StateSlices;
use std::marker::PhantomData;

/// Combines a tuple of [`SliceReducer`]s into a single [`Reducer`] over the
/// full state, applying each slice reducer **in parallel** via Rayon.
///
/// # Compile-time guarantees
///
/// The state must implement [`crate::StateSlices`] declaring its slice list.
/// At every [`Reducer::reduce`] call, the compiler walks `S::Slices` and
/// matches each slice to a reducer in the tuple by its `Slice` associated
/// type. Two conditions are checked at compile time:
///
/// - **Disjointness** — two reducers targeting the same slice produces an
///   "ambiguous impl" error.
/// - **Completeness** — a state slice with no matching reducer produces a
///   "trait not implemented" error.
///
/// This means the user can never accidentally write a parallel root reducer
/// that would race on the same slice, or miss updating a slice the state
/// declares.
///
/// # Runtime behavior
///
/// Each slice reducer runs on a Rayon worker thread, taking a `&Slice`
/// projection of the (cloned) state. Each produces a new slice and optional
/// side events. The slices are reassembled into a new state and the side
/// events are concatenated **in tuple declaration order** (deterministic,
/// independent of execution order).
///
/// # Bounds
///
/// The reducer tuple, state, and slices must be [`Send`]/[`Sync`] for the
/// parallel execution. See the [`crate::Reducer`] impl bounds for details.
///
/// # Type parameters
///
/// - `L` — the HList of reducers (built from the tuple via [`IntoHList`])
/// - `E` — the shared event type across reducers (inferred from usage)
/// - `Indices` — the HList of positional witnesses used by the compiler to
///   resolve each slice's reducer (inferred from usage)
///
/// Users never write `E` or `Indices` explicitly — they are inferred at the
/// call site via Rust's type system.
///
/// # Example
///
/// ```ignore
/// let root = ParallelRootReducer::new((solar_reducer, battery_reducer, meter_reducer));
/// let output = root.reduce(&state, &event);
/// ```
///
/// # Arity
///
/// Tuples of 1 to 12 [`SliceReducer`]s are supported.
pub struct ParallelRootReducer<L, E, Indices> {
    slice_reducers: L,
    _marker: PhantomData<(E, Indices)>,
}

impl<L, E, Indices> ParallelRootReducer<L, E, Indices> {
    /// Builds a parallel root reducer from a tuple of slice reducers.
    ///
    /// The tuple is converted to an internal HList via [`IntoHList`] so the
    /// recursive trait machinery can walk it at compile time.
    pub fn new<T>(reducers: T) -> Self
    where
        T: IntoHList<Output = L>,
    {
        ParallelRootReducer {
            slice_reducers: reducers.into_hlist(),
            _marker: PhantomData,
        }
    }
}

/// A trait to find a slice reducer from a tuple of slice reducers by its
/// slice type.
pub(crate) trait FindReducerBySlice<TargetSlice, Index> {
    type Reducer;
    fn find(&self) -> &Self::Reducer;
}

/// Base case: the target slice is in the head of the tuple.
impl<HeadReducer, TailReducers, TargetSlice> FindReducerBySlice<TargetSlice, Here>
    for HCons<HeadReducer, TailReducers>
where
    HeadReducer: SliceReducer<Slice = TargetSlice>,
{
    type Reducer = HeadReducer;

    fn find(&self) -> &Self::Reducer {
        &self.head
    }
}

/// Recursive case: the target slice is in the tail of the tuple.
impl<HeadReducer, TailReducers, TargetSlice, TailIndex>
    FindReducerBySlice<TargetSlice, There<TailIndex>> for HCons<HeadReducer, TailReducers>
where
    TailReducers: FindReducerBySlice<TargetSlice, TailIndex>,
{
    type Reducer = <TailReducers as FindReducerBySlice<TargetSlice, TailIndex>>::Reducer;

    fn find(&self) -> &Self::Reducer {
        self.tail.find()
    }
}

pub(crate) trait ApplyReducers<L, S, E, Indices> {
    fn apply(reducers: &L, state: &S, event: &E) -> ReducerOutput<S, E>;
}

/// Implementation when recursion reaches the end of the tuple of slice reducers.
impl<L, S, E> ApplyReducers<L, S, E, HNil> for HNil
where
    S: Clone,
{
    fn apply(_reducers: &L, state: &S, _event: &E) -> ReducerOutput<S, E> {
        ReducerOutput {
            state: state.clone(),
            side_events: None,
        }
    }
}

/// Implementation when recursion continues through the tuple of slice reducers.
impl<L, HeadReducer, S, E, HeadSlice, RestSlices, Index, RestIndices>
    ApplyReducers<L, S, E, HCons<Index, RestIndices>> for HCons<HeadSlice, RestSlices>
where
    L: FindReducerBySlice<HeadSlice, Index, Reducer = HeadReducer> + Sync,
    HeadReducer: SliceReducer<Slice = HeadSlice, Event = E> + Sync,
    S: HasSlice<HeadSlice> + Clone + Send + Sync,
    E: Send + Sync,
    HeadSlice: Send + Sync,
    RestSlices: ApplyReducers<L, S, E, RestIndices>,
{
    fn apply(reducers: &L, state: &S, event: &E) -> ReducerOutput<S, E> {
        let reducer = reducers.find();
        let (head_output, rest_output) = rayon::join(
            || reducer.reduce(state.slice(), event),
            || RestSlices::apply(reducers, state, event),
        );

        let new_state = rest_output.state;
        let new_state = new_state.set_slice(head_output.state);

        let mut side_events = head_output.side_events.unwrap_or_default();
        if let Some(rest_side_events) = rest_output.side_events {
            side_events.extend(rest_side_events);
        }

        ReducerOutput {
            state: new_state,
            side_events: if side_events.is_empty() {
                None
            } else {
                Some(side_events)
            },
        }
    }
}

impl<S, L, E, Indices> Reducer<S> for ParallelRootReducer<L, E, Indices>
where
    S: StateSlices + Clone,
    S::Slices: ApplyReducers<L, S, E, Indices>,
{
    type Event = E;

    fn reduce(&self, state: &S, event: &Self::Event) -> ReducerOutput<S, Self::Event> {
        <S::Slices as ApplyReducers<L, S, E, Indices>>::apply(&self.slice_reducers, state, event)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::fixtures::{
        ClosureSliceReducer,
        Event::{self, *},
        ExampleState, FirstSlice, SecondSlice, ThirdSlice, make_reducer_tuple, make_state, no_op,
        shared_tests,
    };
    use crate::sequential_root_reducer::SequentialRootReducer;
    use crate::{HList, ParallelRootReducer, Reducer, ReducerOutput, SliceReducer, StateSlices};

    impl StateSlices for ExampleState {
        type Slices = HList!(FirstSlice, SecondSlice, ThirdSlice);
    }

    fn make_root_reducer() -> impl Reducer<ExampleState, Event = Event> {
        ParallelRootReducer::new(make_reducer_tuple())
    }

    #[test]
    fn updates_first_slice() {
        shared_tests::updates_first_slice(make_root_reducer());
    }

    #[test]
    fn updates_second_slice() {
        shared_tests::updates_second_slice(make_root_reducer());
    }

    #[test]
    fn updates_third_slice() {
        shared_tests::updates_third_slice(make_root_reducer());
    }

    #[test]
    fn no_side_events_when_none_produced() {
        shared_tests::no_side_events_when_none_produced(make_root_reducer());
    }

    #[test]
    fn aggregates_side_events_in_tuple_order() {
        shared_tests::aggregates_side_events_in_tuple_order(make_root_reducer());
    }

    /// Wall-clock proof of parallel execution: a reducer tuple where each
    /// slice reducer sleeps for the same duration should complete in roughly
    /// one slice's time when run through `ParallelRootReducer`, vs. the sum
    /// of all sleeps when run through `SequentialRootReducer`.
    #[test]
    fn parallel_is_faster_than_sequential() {
        fn make_delayed_reducer_tuple() -> (
            impl SliceReducer<Slice = FirstSlice, Event = Event>,
            impl SliceReducer<Slice = SecondSlice, Event = Event>,
            impl SliceReducer<Slice = ThirdSlice, Event = Event>,
        ) {
            (
                ClosureSliceReducer::new(|slice: &FirstSlice, event: &Event| {
                    std::thread::sleep(Duration::from_millis(100));
                    match event {
                        FirstValueUpdate { value } => ReducerOutput {
                            state: FirstSlice { value: *value },
                            side_events: None,
                        },
                        _ => no_op(slice, event),
                    }
                }),
                ClosureSliceReducer::new(|slice: &SecondSlice, event: &Event| {
                    std::thread::sleep(Duration::from_millis(100));
                    match event {
                        SecondValueUpdate { value } => ReducerOutput {
                            state: SecondSlice { value: *value },
                            side_events: None,
                        },
                        _ => no_op(slice, event),
                    }
                }),
                ClosureSliceReducer::new(|slice: &ThirdSlice, event: &Event| {
                    std::thread::sleep(Duration::from_millis(100));
                    match event {
                        ThirdValueUpdate { value } => ReducerOutput {
                            state: ThirdSlice {
                                value: value.clone(),
                            },
                            side_events: None,
                        },
                        SecondValueUpdate { value: _ } => ReducerOutput {
                            state: slice.clone(),
                            side_events: None,
                        },
                        _ => no_op(slice, event),
                    }
                }),
            )
        }

        let state = make_state();

        let parallel_root_reducer = ParallelRootReducer::new(make_delayed_reducer_tuple());
        let sequential_root_reducer = SequentialRootReducer::new(make_delayed_reducer_tuple());

        let start = Instant::now();
        let _ = parallel_root_reducer.reduce(&state, &SecondValueUpdate { value: 1.5 });
        let parallel_duration = start.elapsed();

        let start = Instant::now();
        let _ = sequential_root_reducer.reduce(&state, &SecondValueUpdate { value: 1.5 });
        let sequential_duration = start.elapsed();

        assert!(
            parallel_duration < sequential_duration / 2,
            "parallel ({parallel_duration:?}) should be at least 2x faster than sequential ({sequential_duration:?})"
        );
    }
}
