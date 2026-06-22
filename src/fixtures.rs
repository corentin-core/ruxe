use crate::{HasSlice, ReducerOutput, SliceReducer, fixtures::Event::*};
use std::marker::PhantomData;

pub(crate) struct ClosureSliceReducer<S, E, F> {
    func: F,
    _marker: PhantomData<(S, E)>,
}

impl<S, E, F> ClosureSliceReducer<S, E, F> {
    pub(crate) fn new(func: F) -> Self {
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

pub(crate) fn no_op<S, E>(state: &S, _: &E) -> ReducerOutput<S, E>
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
        pub(crate) struct $state {
            $(pub $field: $slice,)+
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
pub(crate) struct FirstSlice {
    pub value: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SecondSlice {
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ThirdSlice {
    pub value: String,
}

init_sliced_state!(
    ExampleState,
    first_slice: FirstSlice,
    second_slice: SecondSlice,
    third_slice: ThirdSlice,
);

#[derive(Debug, PartialEq)]
pub(crate) enum Event {
    FirstValueUpdate { value: u32 },
    FirstValueOrderingTest {},
    SecondValueUpdate { value: f64 },
    ThirdValueUpdate { value: String },
    SideEvent1 {},
    SideEvent2 {},
}

pub(crate) fn make_state() -> ExampleState {
    ExampleState {
        first_slice: FirstSlice { value: 1 },
        second_slice: SecondSlice { value: 1.5 },
        third_slice: ThirdSlice {
            value: String::from("Hello, world!"),
        },
    }
}

/// Shared assertions for tests that exercise a [`crate::Reducer<ExampleState>`]
/// over the standard EMS-style scenario. Used by both
/// [`crate::SequentialRootReducer`] and [`crate::ParallelRootReducer`] tests
/// to avoid duplicating the assertions per variant.
pub(crate) mod shared_tests {
    use super::{Event, Event::*, ExampleState, FirstSlice, SecondSlice, ThirdSlice, make_state};
    use crate::Reducer;

    pub(crate) fn updates_first_slice<R: Reducer<ExampleState, Event = Event>>(root_reducer: R) {
        let state = make_state();
        let reducer_output = root_reducer.reduce(&state, &FirstValueUpdate { value: 2 });
        assert_eq!(
            reducer_output.state,
            ExampleState {
                first_slice: FirstSlice { value: 2 },
                ..state
            }
        );
    }

    pub(crate) fn updates_second_slice<R: Reducer<ExampleState, Event = Event>>(root_reducer: R) {
        let state = make_state();
        let reducer_output = root_reducer.reduce(&state, &SecondValueUpdate { value: 1.7 });
        assert_eq!(
            reducer_output.state,
            ExampleState {
                second_slice: SecondSlice { value: 1.7 },
                ..state
            }
        );
    }

    pub(crate) fn updates_third_slice<R: Reducer<ExampleState, Event = Event>>(root_reducer: R) {
        let state = make_state();
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

    pub(crate) fn no_side_events_when_none_produced<R: Reducer<ExampleState, Event = Event>>(
        root_reducer: R,
    ) {
        let state = make_state();
        let reducer_output = root_reducer.reduce(&state, &FirstValueUpdate { value: 2 });
        assert_eq!(reducer_output.side_events, None);
    }

    pub(crate) fn aggregates_side_events_in_tuple_order<R: Reducer<ExampleState, Event = Event>>(
        root_reducer: R,
    ) {
        let state = make_state();
        let reducer_output = root_reducer.reduce(&state, &SecondValueUpdate { value: 1.5 });
        assert_eq!(
            reducer_output.side_events,
            Some(vec![SideEvent1 {}, SideEvent2 {}])
        )
    }
}

pub(crate) fn make_reducer_tuple() -> (
    impl SliceReducer<Slice = FirstSlice, Event = Event>,
    impl SliceReducer<Slice = SecondSlice, Event = Event>,
    impl SliceReducer<Slice = ThirdSlice, Event = Event>,
) {
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
