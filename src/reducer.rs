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

#[cfg(test)]
mod tests {
    use crate::reducer::tests::Event::{FirstValueUpdate, SecondValueUpdate};
    use crate::{HasSlice, ReducerOutput, SliceReducer};

    #[derive(Clone, Debug, PartialEq)]
    struct FirstSlice {
        value: u32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct SecondSlice {
        value: f64,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct ExampleState {
        first_slice: FirstSlice,
        second_slice: SecondSlice,
    }

    impl HasSlice<FirstSlice> for ExampleState {
        fn slice(&self) -> &FirstSlice {
            &self.first_slice
        }

        fn set_slice(mut self, slice: FirstSlice) -> Self {
            self.first_slice = slice;
            self
        }
    }

    impl HasSlice<SecondSlice> for ExampleState {
        fn slice(&self) -> &SecondSlice {
            &self.second_slice
        }

        fn set_slice(mut self, slice: SecondSlice) -> Self {
            self.second_slice = slice;
            self
        }
    }

    enum Event {
        FirstValueUpdate { value: u32 },
        SecondValueUpdate { value: f64 },
    }

    struct FirstSliceReducer {}

    impl SliceReducer for FirstSliceReducer {
        type Event = Event;
        type Slice = FirstSlice;

        fn reduce(
            &self,
            slice: &Self::Slice,
            event: &Self::Event,
        ) -> ReducerOutput<Self::Slice, Self::Event> {
            match event {
                FirstValueUpdate { value } => ReducerOutput {
                    state: FirstSlice { value: *value },
                    side_events: None,
                },
                _ => ReducerOutput {
                    state: slice.clone(),
                    side_events: None,
                },
            }
        }
    }

    struct SecondSliceReducer {}

    impl SliceReducer for SecondSliceReducer {
        type Event = Event;
        type Slice = SecondSlice;

        fn reduce(
            &self,
            slice: &Self::Slice,
            event: &Self::Event,
        ) -> ReducerOutput<Self::Slice, Self::Event> {
            match event {
                SecondValueUpdate { value } => ReducerOutput {
                    state: SecondSlice { value: *value },
                    side_events: None,
                },
                _ => ReducerOutput {
                    state: slice.clone(),
                    side_events: None,
                },
            }
        }
    }

    fn dispatch_event(event: Event, example_state: ExampleState) -> ExampleState {
        let new_state = example_state.clone();
        let first_slice_reducer = FirstSliceReducer {};
        let second_slice_reducer = SecondSliceReducer {};

        let first_slice_update = first_slice_reducer.reduce(example_state.slice(), &event);
        let new_state = new_state.set_slice(first_slice_update.state);
        let second_slice_update = second_slice_reducer.reduce(example_state.slice(), &event);
        let new_state = new_state.set_slice(second_slice_update.state);
        new_state
    }

    fn make_state() -> ExampleState {
        ExampleState {
            first_slice: FirstSlice { value: 1 },
            second_slice: SecondSlice { value: 1.5 },
        }
    }

    #[test]
    fn first_slice_reducer() {
        let state = make_state();

        let new_state = dispatch_event(FirstValueUpdate { value: 2 }, state.clone());
        assert_eq!(
            new_state,
            ExampleState {
                first_slice: FirstSlice { value: 2 },
                ..state
            }
        );
    }

    #[test]
    fn second_slice_reducer() {
        let state = make_state();

        let new_state = dispatch_event(SecondValueUpdate { value: 1.7 }, state.clone());
        assert_eq!(
            new_state,
            ExampleState {
                second_slice: SecondSlice { value: 1.7 },
                ..state
            }
        );
    }
}
