use crate::HasSlice;

/// The output of a reducer.
pub struct ReducerOutput<S, E> {
    /// The modified state.
    pub state: S,
    /// Optional events to re-dispatch through the store.
    pub side_events: Option<Vec<E>>,
}

/// A reducer takes the current state and an event, and returns a [`ReducerOutput`]
/// containing the new state and optionally side events to re-dispatch.
///
/// The reducer does not modify the state in place — it produces a new value
/// that the [`crate::Store`] will use to replace the current state.
pub trait Reducer<S> {
    /// The event type this reducer handles.
    type Event;

    /// Computes a new state and optional side events from the current state and an event.
    fn reduce(&self, state: &S, event: &Self::Event) -> ReducerOutput<S, Self::Event>;
}

/// A reducer takes a slice of the current state and an event, and returns a [`ReducerOutput`]
/// containing the new state slice and optionally side events to re-dispatch.
///
/// The reducer does not modify the state slice in place — it produces a new value
/// that the [`crate::Store`] will use to replace the current state slice.
///
/// Slice reducers are structurally isolated: the `reduce` method only receives
/// `&Self::Slice`, making access to other slices impossible by construction.
pub trait SliceReducer {
    /// The event type this reducer handles.
    type Event;
    /// The slice of global state this reducer operates on.
    type Slice;

    /// Computes a new state slice and optional side events from the current state and an event.
    fn reduce(
        &self,
        slice: &Self::Slice,
        event: &Self::Event,
    ) -> ReducerOutput<Self::Slice, Self::Event>;
}

/// Generates a `Reducer<S>` implementation for a tuple of [`SliceReducer`].
///
/// Invoked once per arity (1 through 12).
///
/// # Bounds generated
///
/// - `S: Clone` — the macro clones `&S` once at the start of `reduce` to chain
///   `set_slice` calls
/// - `Ri: SliceReducer<Event = E>` — all reducers in the tuple must share the
///   same `Event` type
/// - `S: HasSlice<Ri::Slice>` — required for each `Ri` to extract its slice
macro_rules! impl_slice_reducer_tuple {
    ($($idx:tt $t:ident),+) => {
        impl<S, E, $($t,)+> Reducer<S> for ($($t,)+)
        where
            S: Clone,
            $($t: SliceReducer<Event = E>, S: HasSlice<$t::Slice>,)+
        {
            type Event = E;

            fn reduce(&self, state: &S, event: &Self::Event) -> ReducerOutput<S, Self::Event> {
                let mut side_events: Vec<Self::Event> = Vec::new();
                let new_state = state.clone();
                $(
                    let slice_update = self.$idx.reduce(new_state.slice(), event);
                    let new_state = new_state.set_slice(slice_update.state);
                    if let Some(events) = slice_update.side_events {
                        side_events.extend(events);
                    }
                )+
                ReducerOutput{state: new_state, side_events: if side_events.is_empty() { None } else { Some(side_events) }}
            }
        }
    };
}

impl_slice_reducer_tuple!(0 R0);
impl_slice_reducer_tuple!(0 R0, 1 R1);
impl_slice_reducer_tuple!(0 R0, 1 R1, 2 R2);
impl_slice_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3);
impl_slice_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4);
impl_slice_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5);
impl_slice_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6);
impl_slice_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7);
impl_slice_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7, 8 R8);
impl_slice_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7, 8 R8, 9 R9);
impl_slice_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7, 8 R8, 9 R9, 10 R10);
impl_slice_reducer_tuple!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7, 8 R8, 9 R9, 10 R10, 11 R11);

#[cfg(test)]
mod tests {
    use crate::reducer::tests::Event::{
        FirstValueOrderingTest, FirstValueUpdate, SecondValueUpdate, SideEvent1, SideEvent2,
        ThirdValueUpdate,
    };
    use crate::{HasSlice, Reducer, ReducerOutput, SliceReducer};
    use std::marker::PhantomData;

    struct ClosureSliceReducer<S, E, F> {
        func: F,
        _marker: PhantomData<(S, E)>,
    }

    impl<S, E, F> ClosureSliceReducer<S, E, F> {
        fn new(func: F) -> Self {
            Self {
                func,
                _marker: PhantomData,
            }
        }
    }
    impl<S, E, F> SliceReducer for ClosureSliceReducer<S, E, F>
    where
        F: Fn(&S, &E) -> ReducerOutput<S, E>,
    {
        type Event = E;
        type Slice = S;

        fn reduce(&self, slice: &Self::Slice, event: &Self::Event) -> ReducerOutput<S, E> {
            (self.func)(slice, event)
        }
    }

    fn no_op<S, E>(state: &S, _: &E) -> ReducerOutput<S, E>
    where
        S: Clone,
    {
        ReducerOutput {
            state: state.clone(),
            side_events: None,
        }
    }

    macro_rules! init_sliced_state {
        ($state:ident, $($field:ident: $slice:ty),+ $(,)?) => {
            #[derive(Clone, Debug, PartialEq)]
            struct $state {
                $($field: $slice,)+
            }

            $(
                impl HasSlice<$slice> for $state {
                    fn slice(&self) -> &$slice { &self.$field }
                    fn set_slice(mut self, slice: $slice) -> Self {
                        self.$field = slice;
                        self
                    }
                }
            )+
        };
    }

    #[derive(Clone, Debug, PartialEq)]
    struct FirstSlice {
        value: u32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct SecondSlice {
        value: f64,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct ThirdSlice {
        value: String,
    }

    init_sliced_state!(
        ExampleState,
        first_slice: FirstSlice,
        second_slice: SecondSlice,
        third_slice: ThirdSlice,
    );

    #[derive(Debug, PartialEq)]
    enum Event {
        FirstValueUpdate { value: u32 },
        FirstValueOrderingTest {},
        SecondValueUpdate { value: f64 },
        ThirdValueUpdate { value: String },
        SideEvent1 {},
        SideEvent2 {},
    }

    fn make_root_reducer() -> impl Reducer<ExampleState, Event = Event> {
        (
            ClosureSliceReducer::new(|slice: &FirstSlice, event: &Event| match event {
                FirstValueUpdate { value } => ReducerOutput {
                    state: FirstSlice { value: *value },
                    side_events: None,
                },
                _ => no_op(slice, event),
            }),
            ClosureSliceReducer::new(|slice: &SecondSlice, event: &Event| match event {
                SecondValueUpdate { value } => ReducerOutput {
                    state: SecondSlice { value: *value },
                    side_events: Some(vec![SideEvent1 {}]),
                },
                _ => no_op(slice, event),
            }),
            ClosureSliceReducer::new(|slice: &ThirdSlice, event: &Event| match event {
                ThirdValueUpdate { value } => ReducerOutput {
                    state: ThirdSlice {
                        value: value.clone(),
                    },
                    side_events: None,
                },
                SecondValueUpdate { value: _ } => ReducerOutput {
                    state: slice.clone(),
                    side_events: Some(vec![SideEvent2 {}]),
                },
                _ => no_op(slice, event),
            }),
        )
    }

    fn make_state() -> ExampleState {
        ExampleState {
            first_slice: FirstSlice { value: 1 },
            second_slice: SecondSlice { value: 1.5 },
            third_slice: ThirdSlice {
                value: String::from("Hello, world!"),
            },
        }
    }

    #[test]
    fn first_slice_reducer() {
        let state = make_state();
        let root_reducer = make_root_reducer();
        let reducer_output = root_reducer.reduce(&state, &FirstValueUpdate { value: 2 });
        assert_eq!(
            reducer_output.state,
            ExampleState {
                first_slice: FirstSlice { value: 2 },
                ..state
            }
        );
    }

    #[test]
    fn second_slice_reducer() {
        let state = make_state();
        let root_reducer = make_root_reducer();
        let reducer_output = root_reducer.reduce(&state, &SecondValueUpdate { value: 1.7 });
        assert_eq!(
            reducer_output.state,
            ExampleState {
                second_slice: SecondSlice { value: 1.7 },
                ..state
            }
        );
    }

    #[test]
    fn third_slice_reducer() {
        let state = make_state();
        let root_reducer = make_root_reducer();
        let reducer_output = root_reducer.reduce(
            &state,
            &ThirdValueUpdate {
                value: String::from("Goodbye, world!"),
            },
        );
        assert_eq!(
            reducer_output.state,
            ExampleState {
                third_slice: ThirdSlice {
                    value: String::from("Goodbye, world!")
                },
                ..state
            }
        );
    }

    #[test]
    fn order_matters() {
        fn make_slice_reducer_set() -> impl SliceReducer<Slice = FirstSlice, Event = Event> {
            ClosureSliceReducer::new(|slice: &FirstSlice, event: &Event| match event {
                FirstValueOrderingTest {} => ReducerOutput {
                    state: FirstSlice { value: 1 },
                    side_events: None,
                },
                _ => no_op(slice, event),
            })
        }

        fn make_slice_reducer_reset() -> impl SliceReducer<Slice = FirstSlice, Event = Event> {
            ClosureSliceReducer::new(|slice: &FirstSlice, event: &Event| match event {
                FirstValueOrderingTest {} => ReducerOutput {
                    state: FirstSlice { value: 0 },
                    side_events: None,
                },
                _ => no_op(slice, event),
            })
        }

        let state = make_state();
        let reducer_output = (make_slice_reducer_set(), make_slice_reducer_reset())
            .reduce(&state, &FirstValueOrderingTest {});
        assert_eq!(
            reducer_output.state,
            ExampleState {
                first_slice: FirstSlice { value: 0 },
                ..state
            }
        );

        let state = make_state();
        let reducer_output = (make_slice_reducer_reset(), make_slice_reducer_set())
            .reduce(&state, &FirstValueOrderingTest {});
        assert_eq!(
            reducer_output.state,
            ExampleState {
                first_slice: FirstSlice { value: 1 },
                ..state
            }
        );
    }

    #[test]
    fn no_side_events_when_none_produced() {
        let state = make_state();
        let root_reducer = make_root_reducer();
        let reducer_output = root_reducer.reduce(&state, &FirstValueUpdate { value: 2 });
        assert_eq!(reducer_output.side_events, None);
    }

    #[test]
    fn side_events_aggregation() {
        let root_reducer = make_root_reducer();
        let state = make_state();
        let reducer_output = root_reducer.reduce(&state, &SecondValueUpdate { value: 1.5 });
        assert_eq!(
            reducer_output.side_events,
            Some(vec![SideEvent1 {}, SideEvent2 {}])
        )
    }
}
