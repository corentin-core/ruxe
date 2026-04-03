use crate::reducer::{Reducer, ReducerOutput};
use std::collections::VecDeque;

/// Errors that can occur during event dispatch.
#[derive(Debug, PartialEq)]
pub enum DispatchError {
    /// The maximum depth of events dispatching is exceeded
    MaxDepthExceeded { depth: usize, max: usize },
}

/// An event pending for processing.
struct PendingEvent<E> {
    /// The actual event.
    event: E,
    /// Depth in store dispatch call stack.
    depth: usize,
}

/// A typed state container that dispatches events through a reducer.
///
/// The store owns the state and the reducer. State can only be modified
/// by dispatching events, ensuring a predictable unidirectional data flow.
pub struct Store<S, R, E> {
    state: S,
    reducer: R,
    queue: VecDeque<PendingEvent<E>>,

    max_depth: usize,
}

impl<S, R: Reducer<S, Event = E>, E> Store<S, R, E> {
    /// Creates a new store with the given initial state and reducer.
    pub fn new(state: S, reducer: R, max_depth: usize) -> Self {
        Store {
            state,
            reducer,
            queue: VecDeque::new(),
            max_depth,
        }
    }

    /// Dispatches an event through the reducer, updating the state.
    ///
    /// Side events produced by the reducer are automatically re-dispatched
    /// in FIFO order. Returns an error if the re-dispatch depth exceeds
    /// `max_depth`.
    pub fn dispatch(&mut self, event: R::Event) -> Result<(), DispatchError> {
        let output = self.reducer.reduce(&self.state, event);
        self.apply_output(output, 0)?;

        while let Some(pending) = self.queue.pop_front() {
            self.internal_dispatch(pending)?
        }
        Ok(())
    }

    /// Returns a reference to the current state.
    pub fn state(&self) -> &S {
        &self.state
    }

    fn apply_output(
        &mut self,
        output: ReducerOutput<S, R::Event>,
        current_depth: usize,
    ) -> Result<(), DispatchError> {
        self.state = output.state;
        if let Some(events) = output.side_events {
            if current_depth >= self.max_depth {
                return Err(DispatchError::MaxDepthExceeded {
                    depth: current_depth,
                    max: self.max_depth,
                });
            }

            for event in events {
                self.queue.push_back(PendingEvent {
                    event,
                    depth: current_depth + 1,
                })
            }
        }
        Ok(())
    }

    fn internal_dispatch(
        &mut self,
        pending_event: PendingEvent<R::Event>,
    ) -> Result<(), DispatchError> {
        let event = pending_event.event;
        let output = self.reducer.reduce(&self.state, event);
        self.apply_output(output, pending_event.depth)
    }
}

#[cfg(test)]
mod tests {
    use crate::reducer::{Reducer, ReducerOutput};
    use crate::store::tests::Event::{
        FirstValueUpdate, IgnoredUpdate, MaximumDepthExceededEvent, MultipleDepthSideEvent,
        SecondValueUpdate, SideEvent,
    };
    use crate::store::{DispatchError, Store};

    enum Event {
        FirstValueUpdate { value: f64 },
        SecondValueUpdate { value: u32 },
        IgnoredUpdate {},
        SideEvent {},
        MultipleDepthSideEvent {},
        MaximumDepthExceededEvent {},
    }

    #[derive(Clone, Debug, PartialEq)]
    struct SimpleState {
        first_value: f64,
        second_value: u32,
    }

    struct SimpleReducer {}

    impl Reducer<SimpleState> for SimpleReducer {
        type Event = Event;

        fn reduce(
            &self,
            state: &SimpleState,
            event: Self::Event,
        ) -> ReducerOutput<SimpleState, Event> {
            match event {
                FirstValueUpdate { value } => ReducerOutput {
                    state: SimpleState {
                        first_value: value,
                        ..*state
                    },
                    side_events: None,
                },
                SecondValueUpdate { value } => ReducerOutput {
                    state: SimpleState {
                        second_value: value,
                        ..*state
                    },
                    side_events: None,
                },
                SideEvent {} => ReducerOutput {
                    state: state.clone(),
                    side_events: Some(vec![SecondValueUpdate { value: 2 }, IgnoredUpdate {}]),
                },
                MultipleDepthSideEvent {} => ReducerOutput {
                    state: state.clone(),
                    side_events: Some(vec![SideEvent {}, FirstValueUpdate { value: 2.0 }]),
                },
                MaximumDepthExceededEvent {} => ReducerOutput {
                    state: state.clone(),
                    side_events: Some(vec![MaximumDepthExceededEvent {}]),
                },
                _ => ReducerOutput {
                    state: state.clone(),
                    side_events: None,
                },
            }
        }
    }

    fn make_state() -> SimpleState {
        SimpleState {
            first_value: 1.5,
            second_value: 3,
        }
    }

    fn make_store() -> Store<SimpleState, SimpleReducer, Event> {
        let reducer = SimpleReducer {};
        let state = make_state();
        Store::new(state.clone(), reducer, 10)
    }

    #[test]
    fn initial_state() {
        let store = make_store();
        let state = make_state();

        assert_eq!(*store.state(), state);
    }

    #[test]
    fn single_dispatch() {
        let mut store = make_store();
        let state = make_state();

        store
            .dispatch(FirstValueUpdate { value: 1.2 })
            .expect("dispatch failed");
        assert_eq!(
            *store.state(),
            SimpleState {
                first_value: 1.2,
                ..state
            }
        );
    }

    #[test]
    fn multiple_dispatch() {
        let mut store = make_store();
        let state = make_state();

        store
            .dispatch(FirstValueUpdate { value: 1.2 })
            .expect("Should not panic");
        assert_eq!(
            *store.state(),
            SimpleState {
                first_value: 1.2,
                ..state
            }
        );

        let state = store.state().clone();
        store
            .dispatch(SecondValueUpdate { value: 5 })
            .expect("Should not panic");
        assert_eq!(
            *store.state(),
            SimpleState {
                second_value: 5,
                ..state
            }
        );
    }

    #[test]
    fn unrelated_dispatch() {
        let mut store = make_store();
        let state = make_state();

        store.dispatch(IgnoredUpdate {}).expect("Should not panic");
        assert_eq!(*store.state(), state);
    }

    #[test]
    fn side_event_dispatch() {
        let mut store = make_store();
        let state = make_state();

        store.dispatch(SideEvent {}).expect("Should not panic");
        assert_eq!(
            *store.state(),
            SimpleState {
                second_value: 2,
                ..state
            }
        )
    }

    #[test]
    fn multiple_side_event_dispatch() {
        let mut store = make_store();

        store
            .dispatch(MultipleDepthSideEvent {})
            .expect("Should not panic");
        assert_eq!(
            *store.state(),
            SimpleState {
                first_value: 2.0,
                second_value: 2,
            }
        )
    }

    #[test]
    fn max_depth_dispatch() {
        let mut store = make_store();

        let err = store
            .dispatch(MaximumDepthExceededEvent {})
            .expect_err("Should return an error");
        assert_eq!(err, DispatchError::MaxDepthExceeded { depth: 10, max: 10 });
    }
}
