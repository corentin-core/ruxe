//! Async dispatch: push events into a [`Store`] from any task.
//!
//! [`init_actor_loop`] pairs a [`DispatchHandle`] (the sending side) with an
//! [`ActorLoop`] that owns the store and serializes every dispatch. The send
//! error types live here too.

use crate::{DispatchError, Store};
use futures::StreamExt;
use futures::channel::mpsc::{Receiver, Sender};
use futures::future::poll_fn;
use std::error::Error;
use std::fmt::{Debug, Display};

/// The bounded channel had no spare capacity — transient, a later attempt
/// may succeed. Carries the rejected event ([`Self::into_inner`]).
///
/// Only [`DispatchHandle::try_dispatch`] fails this way; the back-pressure
/// [`dispatch`](DispatchHandle::dispatch) waits for capacity instead.
#[derive(Debug)]
pub struct Full<E> {
    event: E,
}

impl<E> Full<E> {
    /// Recovers the rejected event.
    #[must_use]
    pub fn into_inner(self) -> E {
        self.event
    }
}

impl<E> Display for Full<E>
where
    E: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Channel full, event rejected: {:?}", self.event)
    }
}

impl<E> Error for Full<E> where E: Debug {}

/// The store's actor loop is gone — permanent, the event was not sent and
/// never will be. Carries the rejected event ([`Self::into_inner`]).
#[derive(Debug)]
pub struct Closed<E> {
    event: E,
}

impl<E> Closed<E> {
    /// Recovers the rejected event.
    #[must_use]
    pub fn into_inner(self) -> E {
        self.event
    }
}

impl<E> Display for Closed<E>
where
    E: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Channel closed, event rejected: {:?}", self.event)
    }
}

impl<E> Error for Closed<E> where E: Debug {}

/// Failure of [`DispatchHandle::try_dispatch`]: [`Full`] (transient) or
/// [`Closed`] (permanent).
#[derive(Debug)]
pub enum TrySend<E> {
    /// See [`Full`].
    Full(Full<E>),
    /// See [`Closed`].
    Closed(Closed<E>),
}

impl<E> Display for TrySend<E>
where
    E: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrySend::Full(err) => std::fmt::Display::fmt(err, f),
            TrySend::Closed(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl<E> Error for TrySend<E> where E: Debug {}

/// A cloneable, [`Send`] handle for pushing events into the store's actor
/// loop from any task.
///
/// Cloning does not require `E: Clone` — only the channel handle is cloned.
pub struct DispatchHandle<E> {
    sender: Sender<E>,
}

impl<E> Clone for DispatchHandle<E> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<E> DispatchHandle<E> {
    /// Sends an event without blocking: fails with [`TrySend::Full`] at
    /// capacity, [`TrySend::Closed`] if the loop is gone.
    pub fn try_dispatch(&mut self, event: E) -> Result<(), TrySend<E>> {
        match self.sender.try_send(event) {
            Ok(()) => Ok(()),
            Err(err) => {
                if err.is_disconnected() {
                    Err(TrySend::Closed(Closed {
                        event: err.into_inner(),
                    }))
                } else {
                    Err(TrySend::Full(Full {
                        event: err.into_inner(),
                    }))
                }
            }
        }
    }

    /// Sends an event, awaiting free capacity if the channel is full
    /// (back-pressure). Fails only with [`Closed`].
    pub async fn dispatch(&mut self, event: E) -> Result<(), Closed<E>> {
        // Not `SinkExt::send`: its `SendError` discards the event, and we need
        // it back to uphold the "event returned inside the error" contract.
        match poll_fn(|cx| self.sender.poll_ready(cx)).await {
            Ok(()) => (),
            Err(_) => {
                return Err(Closed { event });
            }
        }
        match self.sender.try_send(event) {
            Ok(()) => Ok(()),
            // When poll_ready returns Ok, try_send should never fail with Full, so we only handle the Closed case here.
            Err(err) => Err(Closed {
                event: err.into_inner(),
            }),
        }
    }
}

/// Sole owner of a [`Store`]: drains the channel and dispatches each event
/// serially — no locks, no other task ever touches the state.
///
/// Created by [`init_actor_loop`]; driven by [`run`](ActorLoop::run).
pub struct ActorLoop<S, E> {
    store: Store<S, E>,
    receiver: Receiver<E>,
}

impl<S, E> ActorLoop<S, E> {
    /// Drives the store until every [`DispatchHandle`] is dropped, then
    /// returns it.
    ///
    /// Events are dispatched in arrival order; on channel close the buffered
    /// ones are drained first — none are lost. A dispatch error (e.g.
    /// `MaxDepthExceeded`) stops the loop and drops the store.
    ///
    /// The future is **not** `Send` (the middleware chain is a plain
    /// `Box<dyn FnMut>`): await it on the current task — it cannot be
    /// `tokio::spawn`ed.
    pub async fn run(mut self) -> Result<Store<S, E>, DispatchError> {
        while let Some(event) = self.receiver.next().await {
            self.store.dispatch(event)?
        }
        Ok(self.store)
    }
}

/// Wraps a [`Store`] in an [`ActorLoop`] and returns it alongside a
/// [`DispatchHandle`] for pushing events into it.
///
/// `max_event_buffer` sizes the bounded channel between them. Note the
/// effective capacity is `max_event_buffer + number of live handles` — each
/// handle reserves one guaranteed slot. A full channel applies back-pressure
/// to [`DispatchHandle::dispatch`].
///
/// # Example
///
/// ```ignore
/// let (handle, actor_loop) = init_actor_loop(store, 32);
///
/// // Move cloned handles into your own tasks — ruxe never spawns.
/// let mut producer = handle.clone();
/// tokio::spawn(async move { producer.dispatch(event).await });
/// drop(handle); // run() only ends once every handle is dropped
///
/// // run()'s future is not Send — await it here rather than spawning it.
/// let store = actor_loop.run().await?;
/// ```
#[must_use]
pub fn init_actor_loop<S, E>(
    store: Store<S, E>,
    max_event_buffer: usize,
) -> (DispatchHandle<E>, ActorLoop<S, E>) {
    let (sender, receiver) = futures::channel::mpsc::channel(max_event_buffer);
    let handle = DispatchHandle { sender };
    let actor_loop = ActorLoop { store, receiver };
    (handle, actor_loop)
}

#[cfg(test)]
mod tests {

    mod dispatch_handle {

        use crate::actor::{Closed, DispatchHandle, TrySend};
        use futures::StreamExt;
        use futures::{executor::block_on, join};

        #[test]
        fn dispatch_handle_try_dispatch_happy_path() {
            let (sender, _receiver) = futures::channel::mpsc::channel(1);
            let mut handle = DispatchHandle { sender };
            let event = 42;
            let result = handle.try_dispatch(event);
            result.expect("Dispatch should succeed");
        }

        #[test]
        fn dispatch_handle_try_dispatch_full() {
            let (sender, _receiver) = futures::channel::mpsc::channel(0); // Channel capacity is 1 (0 + number of senders = 1)
            let mut handle = DispatchHandle { sender };
            let event1 = 42;
            let event2 = 43;
            handle
                .try_dispatch(event1)
                .expect("First dispatch should succeed");
            let result = handle.try_dispatch(event2);
            match result {
                Err(TrySend::Full(err)) => {
                    let rejected_event = err.into_inner();
                    assert_eq!(rejected_event, event2);
                }
                other => panic!("expected Full, got {other:?}"),
            }
        }

        #[test]
        fn dispatch_handle_try_dispatch_closed() {
            let (sender, receiver) = futures::channel::mpsc::channel(1);
            drop(receiver); // Close the receiver
            let mut handle = DispatchHandle { sender };
            let event = 42;
            let result = handle.try_dispatch(event);
            match result {
                Err(TrySend::Closed(err)) => {
                    let rejected_event = err.into_inner();
                    assert_eq!(rejected_event, event);
                }
                other => panic!("expected Closed, got {other:?}"),
            }
        }

        #[test]
        fn dispatch_handle_dispatch_happy_path() {
            let (sender, mut receiver) = futures::channel::mpsc::channel(1);
            let mut handle = DispatchHandle { sender };
            let event = 42;
            let dispatch_future = handle.dispatch(event);
            let received_event = block_on(async {
                let dispatch_result = dispatch_future.await;
                dispatch_result.expect("Dispatch should succeed");
                receiver.next().await.expect("Should receive the event")
            });
            assert_eq!(received_event, 42);
        }

        #[test]
        fn dispatch_handle_dispatch_closed() {
            let (sender, receiver) = futures::channel::mpsc::channel(1);
            drop(receiver); // Close the receiver
            let mut handle = DispatchHandle { sender };
            let event = 42;
            let dispatch_future = handle.dispatch(event);
            let result = block_on(dispatch_future);
            assert!(matches!(result, Err(Closed { event: 42 })));
        }

        #[test]
        fn dispatch_handle_dispatch_back_pressure() {
            let (sender, receiver) = futures::channel::mpsc::channel(0);
            let mut handle = DispatchHandle { sender };
            let event1 = 42;
            let event2 = 43;
            let dispatch_future = async move {
                handle
                    .dispatch(event1)
                    .await
                    .expect("First dispatch should succeed");
                handle
                    .dispatch(event2)
                    .await
                    .expect("Second dispatch should succeed");
            };
            block_on(async {
                let _ = join!(dispatch_future, async {
                    let received_events: Vec<_> = receiver.take(2).collect().await;
                    assert!(received_events.contains(&42));
                    assert!(received_events.contains(&43));
                    assert_eq!(received_events.len(), 2);
                });
            });
        }

        #[test]
        fn dispatch_handle_clone() {
            let (sender, receiver) = futures::channel::mpsc::channel(1);
            let mut handle1 = DispatchHandle { sender };
            let mut handle2 = handle1.clone();
            let event1 = 42;
            let event2 = 43;
            let dispatch_future1 = async move {
                handle1
                    .dispatch(event1)
                    .await
                    .expect("First dispatch should succeed");
            };
            let dispatch_future2 = async move {
                handle2
                    .dispatch(event2)
                    .await
                    .expect("Second dispatch should succeed");
            };
            block_on(async {
                let (_, _) = join!(dispatch_future1, dispatch_future2);
                let received_events: Vec<_> = receiver.take(2).collect().await;
                assert!(received_events.contains(&42));
                assert!(received_events.contains(&43));
                assert_eq!(received_events.len(), 2);
            });
        }
    }

    mod actor_loop {
        use crate::{DispatchError, Reducer, ReducerOutput, Store, init_actor_loop};
        use futures::executor::block_on;

        use Event::*;

        #[derive(Debug)]
        pub(super) enum Event {
            Append { value: char },
            Recursive {},
        }

        #[derive(Clone, Debug, PartialEq)]
        pub(super) struct SimpleState {
            pub(super) name: String,
        }

        pub(super) struct SimpleReducer {}

        impl Reducer<SimpleState> for SimpleReducer {
            type Event = Event;

            fn reduce(
                &self,
                current_state: &SimpleState,
                event: &Self::Event,
            ) -> ReducerOutput<SimpleState, Event> {
                match event {
                    Append { value } => ReducerOutput {
                        state: SimpleState {
                            name: format!("{}{}", current_state.name, *value),
                        },
                        side_events: None,
                    },
                    Recursive {} => ReducerOutput {
                        state: current_state.clone(),
                        side_events: Some(vec![Recursive {}]),
                    },
                }
            }
        }

        pub(super) fn make_state() -> SimpleState {
            SimpleState {
                name: String::new(),
            }
        }

        pub(super) fn make_store() -> Store<SimpleState, Event> {
            let reducer = SimpleReducer {};
            Store::new(make_state(), reducer, vec![], 10)
        }

        #[test]
        fn actor_loop_no_events() {
            let store = make_store();
            let (handle, actor_loop) = init_actor_loop(store, 10);
            drop(handle); // Close the sender to stop the actor loop immediately
            let store = block_on(actor_loop.run()).expect("Actor loop should run successfully");
            assert_eq!(
                store.state(),
                &SimpleState {
                    name: String::new()
                }
            );
        }

        #[test]
        fn actor_loop_multiple_events() {
            let store = make_store();
            let (mut handle, actor_loop) = init_actor_loop(store, 10);
            let events = vec![
                Append { value: 'a' },
                Append { value: 'b' },
                Append { value: 'c' },
            ];
            for event in events {
                handle.try_dispatch(event).expect("Dispatch should succeed");
            }
            drop(handle); // Close the sender to stop the actor loop after processing the events
            let store = block_on(actor_loop.run()).expect("Actor loop should run successfully");
            assert_eq!(
                store.state(),
                &SimpleState {
                    name: "abc".to_string()
                }
            );
        }

        #[test]
        fn actor_loop_max_recursion_depth() {
            let store = make_store();
            let (mut handle, actor_loop) = init_actor_loop(store, 10);
            handle
                .try_dispatch(Event::Recursive {})
                .expect("Dispatch should succeed");
            drop(handle); // Close the sender to stop the actor loop after processing the events
            let result = block_on(actor_loop.run());
            match result {
                Ok(_) => panic!("Actor loop should have failed due to max recursion depth"),
                Err(err) => assert_eq!(err, DispatchError::MaxDepthExceeded { depth: 10, max: 10 }),
            }
        }
    }
}
