//! Parallel root reducer: applies a tuple of [`SliceReducer`]s in parallel
//! via Rayon, with compile-time guarantees that each state slice has exactly
//! one matching reducer. See [`ParallelRootReducer`] for usage.

use crate::hlist::{HCons, HNil, IntoHList};
use crate::indices::{Here, There};
use crate::reducer::{Reducer, ReducerOutput, SliceReducer};
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
/// Each slice reducer runs on a Rayon worker thread, taking a `&mut Slice`
/// projection of the state. Each may update the slice and produce optional
/// side events. The side events are concatenated **in tuple declaration
/// order** (deterministic, independent of execution order).
///
/// # Bounds
///
/// The reducer tuple, state, and slices must be [`Send`]/[`Sync`] for the
/// parallel execution. See the [`crate::Reducer`] impl bounds for details.
///
/// # Type parameters
///
/// - `Reducers` — the HList of reducers (built from the tuple via [`IntoHList`])
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
pub struct ParallelRootReducer<Reducers, E, Indices> {
    slice_reducers: Reducers,
    _marker: PhantomData<(E, Indices)>,
}

impl<Reducers, E, Indices> ParallelRootReducer<Reducers, E, Indices> {
    /// Builds a parallel root reducer from a tuple of slice reducers.
    ///
    /// The tuple is converted to an internal HList via [`IntoHList`] so the
    /// recursive trait machinery can walk it at compile time.
    pub fn new<Tuple>(reducers: Tuple) -> Self
    where
        Tuple: IntoHList<Output = Reducers>,
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

pub(crate) trait ApplyReducers<Reducers, S, E, Indices> {
    fn apply(&mut self, reducers: &Reducers, event: &E) -> ReducerOutput<E>;
}

/// Implementation when recursion reaches the end of the tuple of slice reducers.
impl<Reducers, S, E> ApplyReducers<Reducers, S, E, HNil> for HNil {
    fn apply(&mut self, _reducers: &Reducers, _event: &E) -> ReducerOutput<E> {
        None
    }
}

/// Implementation when recursion continues through the tuple of slice reducers.
impl<Reducers, HeadReducer, S, E, HeadSlice, RestSlices, Index, RestIndices>
    ApplyReducers<Reducers, S, E, HCons<Index, RestIndices>> for HCons<&mut HeadSlice, RestSlices>
where
    Reducers: FindReducerBySlice<HeadSlice, Index, Reducer = HeadReducer> + Sync,
    HeadReducer: SliceReducer<Slice = HeadSlice, Event = E> + Sync,
    E: Send + Sync,
    HeadSlice: Send + Sync,
    RestSlices: ApplyReducers<Reducers, S, E, RestIndices> + Send + Sync,
{
    fn apply(&mut self, reducers: &Reducers, event: &E) -> ReducerOutput<E> {
        let reducer = reducers.find();
        let (head_output, rest_output) = rayon::join(
            || reducer.reduce(self.head, event),
            || self.tail.apply(reducers, event),
        );

        let mut side_events = head_output.unwrap_or_default();
        if let Some(rest_side_events) = rest_output {
            side_events.extend(rest_side_events);
        }

        if side_events.is_empty() {
            None
        } else {
            Some(side_events)
        }
    }
}

impl<S, Reducers, E, Indices> Reducer<S> for ParallelRootReducer<Reducers, E, Indices>
where
    S: StateSlices,
    for<'s> S::Slices<'s>: ApplyReducers<Reducers, S, E, Indices>,
{
    type Event = E;

    fn reduce(&self, state: &mut S, event: &Self::Event) -> ReducerOutput<Self::Event> {
        state.to_slices().apply(&self.slice_reducers, event)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::IntoHList;
    use crate::fixtures::{
        ClosureSliceReducer,
        Event::{self, *},
        ExampleState, FirstSlice, SecondSlice, ThirdSlice, make_reducer_tuple, make_state, no_op,
        shared_tests,
    };
    use crate::sequential_root_reducer::SequentialRootReducer;
    use crate::{HList, ParallelRootReducer, Reducer, SliceReducer, StateSlices};

    impl StateSlices for ExampleState {
        type Slices<'s> = HList!(&'s mut FirstSlice, &'s mut SecondSlice, &'s mut ThirdSlice);

        fn to_slices(&mut self) -> Self::Slices<'_> {
            (
                &mut self.first_slice,
                &mut self.second_slice,
                &mut self.third_slice,
            )
                .into_hlist()
        }
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
                ClosureSliceReducer::new(|slice: &mut FirstSlice, event: &Event| {
                    std::thread::sleep(Duration::from_millis(100));
                    match event {
                        FirstValueUpdate { value } => {
                            slice.value = *value;
                            None
                        }
                        _ => no_op(slice, event),
                    }
                }),
                ClosureSliceReducer::new(|slice: &mut SecondSlice, event: &Event| {
                    std::thread::sleep(Duration::from_millis(100));
                    match event {
                        SecondValueUpdate { value } => {
                            slice.value = *value;
                            None
                        }
                        _ => no_op(slice, event),
                    }
                }),
                ClosureSliceReducer::new(|slice: &mut ThirdSlice, event: &Event| {
                    std::thread::sleep(Duration::from_millis(100));
                    match event {
                        ThirdValueUpdate { value } => {
                            slice.value = value.clone();
                            None
                        }
                        SecondValueUpdate { value: _ } => None,
                        _ => no_op(slice, event),
                    }
                }),
            )
        }

        let mut state = make_state();

        let parallel_root_reducer = ParallelRootReducer::new(make_delayed_reducer_tuple());
        let sequential_root_reducer = SequentialRootReducer::new(make_delayed_reducer_tuple());

        let start = Instant::now();
        let _ = parallel_root_reducer.reduce(&mut state, &SecondValueUpdate { value: 1.5 });
        let parallel_duration = start.elapsed();

        let start = Instant::now();
        let _ = sequential_root_reducer.reduce(&mut state, &SecondValueUpdate { value: 1.5 });
        let sequential_duration = start.elapsed();

        assert!(
            parallel_duration < sequential_duration / 2,
            "parallel ({parallel_duration:?}) should be at least 2x faster than sequential ({sequential_duration:?})"
        );
    }
}
