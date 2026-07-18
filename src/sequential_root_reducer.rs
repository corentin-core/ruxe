//! Sequential root reducer: applies a tuple of [`SliceReducer`]s in
//! declaration order, threading the state through chained `set_slice`
//! calls. See [`SequentialRootReducer`] for usage.

use crate::{HasSlice, Reducer, ReducerOutput, SliceReducer};

/// Combines a tuple of [`SliceReducer`]s into a single [`Reducer`] over the
/// full state, applying each slice reducer **in declaration order**.
///
/// # Behavior
///
/// On each [`Reducer::reduce`] call, the state is cloned once, then each
/// slice reducer is invoked in order. Each reducer's output replaces the
/// corresponding slice (via [`HasSlice::set_slice`]) before the next reducer
/// runs. Side events from all reducers are concatenated in tuple order.
///
/// # When to use
///
/// - When two reducers may target the same slice and you want the later one
///   to win (e.g. layered logic).
/// - When the state type doesn't implement [`crate::StateSlices`] (i.e. you
///   haven't declared the slice list).
///
/// For independent, disjoint slice reducers with parallel execution, use
/// [`crate::ParallelRootReducer`] instead.
///
/// # Example
///
/// ```ignore
/// let root = SequentialRootReducer::new((solar_reducer, battery_reducer, meter_reducer));
/// let output = root.reduce(&state, &event);
/// ```
///
/// # Arity
///
/// Tuples of 1 to 12 [`SliceReducer`]s are supported.
pub struct SequentialRootReducer<Reducers> {
    reducers: Reducers,
}

impl<Reducers> SequentialRootReducer<Reducers> {
    /// Builds a sequential root reducer from a tuple of slice reducers.
    pub fn new(reducers: Reducers) -> Self {
        SequentialRootReducer { reducers }
    }
}

/// Generates `Reducer<S>` for `SequentialRootReducer<(R0, R1, ...)>`, invoked per arity.
///
/// Bounds generated:
/// - `Ri: SliceReducer<Event = E>` — shared `Event` type across the tuple
/// - `S: HasSlice<Ri::Slice>` — required per `Ri` to extract its slice
macro_rules! impl_sequential_reducer_tuple {
    ($($idx:tt $t:ident),+) => {
        impl<S, E, $($t,)+> Reducer<S> for SequentialRootReducer<($($t,)+)>
        where
            $($t: SliceReducer<Event = E>, S: HasSlice<$t::Slice>,)+
        {
            type Event = E;

            fn reduce(&self, state: &mut S, event: &Self::Event) -> ReducerOutput<Self::Event> {
                let mut side_events: Vec<Self::Event> = Vec::new();

                $(
                    let new_side_events = self.reducers.$idx.reduce(state.slice(), event);
                    if let Some(events) = new_side_events {
                        side_events.extend(events);
                    }
                )+

                if side_events.is_empty() { None } else { Some(side_events) }
            }
        }
    };
}

impl_sequential_reducer_tuple!(0 R0);
impl_sequential_reducer_tuple!(0 R0, 1 R1);
impl_sequential_reducer_tuple!(0 R0, 1 R1, 2 R2);
impl_sequential_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3);
impl_sequential_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4);
impl_sequential_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5);
impl_sequential_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6);
impl_sequential_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7);
impl_sequential_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7, 8 R8);
impl_sequential_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7, 8 R8, 9 R9);
impl_sequential_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7, 8 R8, 9 R9, 10 R10);
impl_sequential_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7, 8 R8, 9 R9, 10 R10, 11 R11);

#[cfg(test)]
mod tests {
    use crate::fixtures::{
        ClosureSliceReducer,
        Event::{self, *},
        ExampleState, FirstSlice, make_reducer_tuple, make_state, no_op, shared_tests,
    };
    use crate::sequential_root_reducer::SequentialRootReducer;
    use crate::{Reducer, SliceReducer};

    fn make_root_reducer() -> impl Reducer<ExampleState, Event = Event> {
        SequentialRootReducer::new(make_reducer_tuple())
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

    /// `SequentialRootReducer` applies reducers in tuple order: when two
    /// reducers target the same slice, the later one wins (vs. a compile
    /// error in `ParallelRootReducer`).
    #[test]
    fn last_reducer_wins_for_duplicate_slice() {
        fn make_slice_reducer_set() -> impl SliceReducer<Slice = FirstSlice, Event = Event> {
            ClosureSliceReducer::new(|slice: &mut FirstSlice, event: &Event| match event {
                FirstValueOrderingTest {} => {
                    slice.value = 1;
                    None
                }
                _ => no_op(slice, event),
            })
        }

        fn make_slice_reducer_reset() -> impl SliceReducer<Slice = FirstSlice, Event = Event> {
            ClosureSliceReducer::new(|slice: &mut FirstSlice, event: &Event| match event {
                FirstValueOrderingTest {} => {
                    slice.value = 0;
                    None
                }
                _ => no_op(slice, event),
            })
        }

        let mut state = make_state();
        let original = state.clone();
        let _ = SequentialRootReducer::new((make_slice_reducer_set(), make_slice_reducer_reset()))
            .reduce(&mut state, &FirstValueOrderingTest {});
        assert_eq!(
            state,
            ExampleState {
                first_slice: FirstSlice { value: 0 },
                ..original
            }
        );

        let mut state = make_state();
        let original = state.clone();
        let _ = SequentialRootReducer::new((make_slice_reducer_reset(), make_slice_reducer_set()))
            .reduce(&mut state, &FirstValueOrderingTest {});
        assert_eq!(
            state,
            ExampleState {
                first_slice: FirstSlice { value: 1 },
                ..original
            }
        );
    }
}
