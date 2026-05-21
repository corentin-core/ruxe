//! Event-driven state container. The [`Store`] owns the state and the
//! composed middleware chain, and exposes [`Store::dispatch`] as the only
//! way to evolve the state.

use crate::middleware::{Middleware, Next};
use crate::reducer::Reducer;
use std::collections::VecDeque;

/// Errors returned by [`Store::dispatch`].
#[derive(Debug, PartialEq)]
pub enum DispatchError {
    /// Side-event recursion exceeded `max_depth`.
    MaxDepthExceeded { depth: usize, max: usize },
}

struct PendingEvent<E> {
    event: E,
    /// Re-dispatch depth (0 = original dispatch).
    depth: usize,
}

/// A typed state container that dispatches events through a middleware
/// chain and a reducer.
///
/// State changes only via [`dispatch`](Store::dispatch). The reducer is
/// the sole producer of new state; middlewares observe, transform, or
/// short-circuit the flow.
pub struct Store<S, E> {
    state: S,
    queue: VecDeque<PendingEvent<E>>,
    chain: Next<S, E>,
    max_depth: usize,
}

impl<S, E> Store<S, E> {
    /// Builds a store from an initial state, a reducer, and an ordered list
    /// of middlewares (see [`crate::middleware`] for the composition order).
    ///
    /// `max_depth` bounds the side-event re-dispatch depth (original dispatch
    /// is depth 0). An event at depth `max_depth` may run but cannot emit
    /// further side events.
    pub fn new<R>(
        state: S,
        reducer: R,
        middlewares: Vec<Box<dyn Middleware<S, E>>>,
        max_depth: usize,
    ) -> Self
    where
        R: Reducer<S, Event = E> + 'static,
        S: 'static,
        E: 'static,
    {
        let base = Box::new(move |state: &mut S, event: E| -> Option<Vec<E>> {
            let output = reducer.reduce(state, &event);
            *state = output.state;
            output.side_events
        });
        Store {
            state,
            chain: middlewares
                .into_iter()
                .rev()
                .fold(base, |res, mid| mid.wrap(res)),
            queue: VecDeque::new(),
            max_depth,
        }
    }

    /// Dispatches an event through the chain. Side events are re-dispatched
    /// in FIFO order; errors with [`DispatchError::MaxDepthExceeded`] when
    /// recursion exceeds `max_depth`.
    pub fn dispatch(&mut self, event: E) -> Result<(), DispatchError> {
        let events = (self.chain)(&mut self.state, event);
        self.queue_side_events(events, 0)?;

        while let Some(pending) = self.queue.pop_front() {
            self.internal_dispatch(pending)?
        }
        Ok(())
    }

    /// Returns a reference to the current state.
    pub fn state(&self) -> &S {
        &self.state
    }

    fn queue_side_events(
        &mut self,
        events: Option<Vec<E>>,
        current_depth: usize,
    ) -> Result<(), DispatchError> {
        if let Some(events) = events {
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

    fn internal_dispatch(&mut self, pending_event: PendingEvent<E>) -> Result<(), DispatchError> {
        let event = pending_event.event;
        let events = (self.chain)(&mut self.state, event);
        self.queue_side_events(events, pending_event.depth)
    }
}

#[cfg(test)]
mod tests {

    mod fixtures {
        use crate::{Reducer, ReducerOutput, Store};
        use Event::*;
        pub(super) enum Event {
            FirstValueUpdate { value: f64 },
            SecondValueUpdate { value: u32 },
            IgnoredUpdate {},
            EmitSide {},
            EmitNestedSide {},
            EmitSelfRecursive {},
        }

        #[derive(Clone, Debug, PartialEq)]
        pub(super) struct SimpleState {
            pub(super) first_value: f64,
            pub(super) second_value: u32,
        }

        pub(super) struct SimpleReducer {}

        impl Reducer<SimpleState> for SimpleReducer {
            type Event = Event;

            fn reduce(
                &self,
                state: &SimpleState,
                event: &Self::Event,
            ) -> ReducerOutput<SimpleState, Event> {
                match event {
                    FirstValueUpdate { value } => ReducerOutput {
                        state: SimpleState {
                            first_value: *value,
                            ..*state
                        },
                        side_events: None,
                    },
                    SecondValueUpdate { value } => ReducerOutput {
                        state: SimpleState {
                            second_value: *value,
                            ..*state
                        },
                        side_events: None,
                    },
                    EmitSide {} => ReducerOutput {
                        state: state.clone(),
                        side_events: Some(vec![SecondValueUpdate { value: 2 }, IgnoredUpdate {}]),
                    },
                    EmitNestedSide {} => ReducerOutput {
                        state: state.clone(),
                        side_events: Some(vec![EmitSide {}, FirstValueUpdate { value: 2.0 }]),
                    },
                    EmitSelfRecursive {} => ReducerOutput {
                        state: state.clone(),
                        side_events: Some(vec![EmitSelfRecursive {}]),
                    },
                    _ => ReducerOutput {
                        state: state.clone(),
                        side_events: None,
                    },
                }
            }
        }

        pub(super) fn make_state() -> SimpleState {
            SimpleState {
                first_value: 1.5,
                second_value: 3,
            }
        }

        pub(super) fn make_store() -> Store<SimpleState, Event> {
            let reducer = SimpleReducer {};
            Store::new(make_state(), reducer, vec![], 10)
        }
    }

    mod basic_dispatch {
        use super::fixtures::Event::*;
        use super::fixtures::*;

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
    }

    mod side_events {
        use super::fixtures::Event::*;
        use super::fixtures::*;
        use crate::store::DispatchError;
        #[test]
        fn side_event_dispatch() {
            let mut store = make_store();
            let state = make_state();

            store.dispatch(EmitSide {}).expect("Should not panic");
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

            store.dispatch(EmitNestedSide {}).expect("Should not panic");
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
                .dispatch(EmitSelfRecursive {})
                .expect_err("Should return an error");
            assert_eq!(err, DispatchError::MaxDepthExceeded { depth: 10, max: 10 });
        }
    }

    mod middlewares {
        use super::fixtures::Event::*;
        use super::fixtures::{Event, SimpleState, make_state};
        use crate::Middleware;
        use fixtures::*;
        use std::rc::Rc;

        mod fixtures {
            use super::super::fixtures::{Event, SimpleReducer, SimpleState, make_state};
            use crate::{Middleware, Next, Store};
            use std::cell::RefCell;
            use std::marker::PhantomData;
            use std::rc::Rc;

            pub(super) struct Logger {
                pub logs: Vec<String>,
            }

            impl Logger {
                fn new() -> Self {
                    Self { logs: vec![] }
                }

                pub(super) fn log(&mut self, msg: impl Into<String>) {
                    self.logs.push(msg.into());
                }
            }

            pub(super) fn make_shared_logger() -> Rc<RefCell<Logger>> {
                Rc::new(RefCell::new(Logger::new()))
            }

            pub(super) struct ProbeMiddleware<S, E> {
                pre: Box<dyn FnMut(&S)>,
                post: Box<dyn FnMut(&S)>,
                _marker: PhantomData<E>,
            }

            impl<S, E> ProbeMiddleware<S, E> {
                pub(super) fn new(pre: Box<dyn FnMut(&S)>, post: Box<dyn FnMut(&S)>) -> Self {
                    Self {
                        pre,
                        post,
                        _marker: PhantomData,
                    }
                }
            }

            impl Middleware<SimpleState, Event> for ProbeMiddleware<SimpleState, Event> {
                fn wrap(
                    mut self: Box<Self>,
                    mut next: Next<SimpleState, Event>,
                ) -> Next<SimpleState, Event> {
                    Box::new(move |state, event| {
                        (self.pre)(state);
                        let output = next(state, event);
                        (self.post)(state);
                        output
                    })
                }
            }

            pub(super) struct ShortCircuitMiddleware {}

            impl Middleware<SimpleState, Event> for ShortCircuitMiddleware {
                fn wrap(self: Box<Self>, _: Next<SimpleState, Event>) -> Next<SimpleState, Event> {
                    Box::new(|_, _| None)
                }
            }

            pub(super) fn make_store(
                middlewares: Vec<Box<dyn Middleware<SimpleState, Event>>>,
            ) -> Store<SimpleState, Event> {
                let reducer = SimpleReducer {};
                Store::new(make_state(), reducer, middlewares, 10)
            }
        }

        #[test]
        fn onion_order() {
            let logger = make_shared_logger();
            let push_msg = |msg: &'static str| -> Box<dyn FnMut(&SimpleState)> {
                let logger_copy = Rc::clone(&logger);
                Box::new(move |_| logger_copy.borrow_mut().log(msg))
            };

            let middlewares: Vec<Box<dyn Middleware<SimpleState, Event>>> = vec![
                Box::new(ProbeMiddleware::<SimpleState, Event>::new(
                    push_msg("Pre-Middleware1"),
                    push_msg("Post-Middleware1"),
                )),
                Box::new(ProbeMiddleware::<SimpleState, Event>::new(
                    push_msg("Pre-Middleware2"),
                    push_msg("Post-Middleware2"),
                )),
            ];

            let mut store = make_store(middlewares);

            store
                .dispatch(FirstValueUpdate { value: 1.2 })
                .expect("dispatch failed");

            assert_eq!(
                *logger.borrow().logs,
                vec![
                    "Pre-Middleware1",
                    "Pre-Middleware2",
                    "Post-Middleware2",
                    "Post-Middleware1",
                ]
            )
        }
        #[test]
        fn inspect_state_pre_post_dispatch() {
            let logger = make_shared_logger();
            let inspect_state = |step: &'static str| -> Box<dyn FnMut(&SimpleState)> {
                let logger_copy = Rc::clone(&logger);
                Box::new(move |state| {
                    logger_copy
                        .borrow_mut()
                        .log(format!("First value {}: {}", step, state.first_value))
                })
            };

            let middlewares: Vec<Box<dyn Middleware<SimpleState, Event>>> =
                vec![Box::new(ProbeMiddleware::<SimpleState, Event>::new(
                    inspect_state("before dispatch"),
                    inspect_state("after dispatch"),
                ))];

            let mut store = make_store(middlewares);

            store
                .dispatch(FirstValueUpdate { value: 1.2 })
                .expect("dispatch failed");

            assert_eq!(
                *logger.borrow().logs,
                vec![
                    "First value before dispatch: 1.5",
                    "First value after dispatch: 1.2",
                ]
            )
        }

        #[test]
        fn short_circuit() {
            let logger = make_shared_logger();
            let push_msg = |msg: &'static str| -> Box<dyn FnMut(&SimpleState)> {
                let logger_copy = Rc::clone(&logger);
                Box::new(move |_| logger_copy.borrow_mut().log(msg))
            };

            let middlewares: Vec<Box<dyn Middleware<SimpleState, Event>>> = vec![
                Box::new(ProbeMiddleware::<SimpleState, Event>::new(
                    push_msg("Pre-Middleware1"),
                    push_msg("Post-Middleware1"),
                )),
                Box::new(ShortCircuitMiddleware {}),
                Box::new(ProbeMiddleware::<SimpleState, Event>::new(
                    push_msg("Pre-Middleware2"),
                    push_msg("Post-Middleware2"),
                )),
            ];

            let mut store = make_store(middlewares);

            store
                .dispatch(FirstValueUpdate { value: 1.2 })
                .expect("dispatch failed");

            assert_eq!(
                *logger.borrow().logs,
                vec!["Pre-Middleware1", "Post-Middleware1",]
            );

            assert_eq!(*store.state(), make_state());
        }
    }
}
