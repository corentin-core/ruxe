//! Async dispatch: push events into a [`Store`] from any task.
//!
//! [`init_actor_loop`] pairs a [`DispatchHandle`] (the sending side) with an
//! [`ActorLoop`] that owns the store and serializes every dispatch. The send
//! error types live here too.

use crate::watch;
use crate::{DispatchError, Store};
use futures::channel::mpsc::{Receiver, Sender};
use futures::future::poll_fn;
use futures::{Stream, StreamExt};
use std::error::Error;
use std::fmt::{Debug, Display};
use std::sync::Arc;

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

///  Sealed trait pattern to prevent downstream crates to implement Publish trait
mod sealed {
    pub trait Sealed {}
}

/// State-publication policy for [`ActorLoop`]. Sealed, so only [`NoPublish`]
/// (the default no-op) and [`WatchPublish`] implement it.
#[doc(hidden)]
pub trait Publish<S>: sealed::Sealed {
    fn publish(&self, state: &S);

    fn has_receivers(&self) -> bool;
}

#[doc(hidden)]
pub struct NoPublish {
    // This is a private field to prevent instantiation outside of this module
    _private: (),
}
impl<S> Publish<S> for NoPublish {
    fn publish(&self, _: &S) {}

    fn has_receivers(&self) -> bool {
        false
    }
}

impl sealed::Sealed for NoPublish {}

#[doc(hidden)]
pub struct WatchPublish<S> {
    publisher: watch::Sender<Arc<S>>,
}

impl<S> sealed::Sealed for WatchPublish<S> {}

impl<S: Clone> Publish<S> for WatchPublish<S> {
    fn publish(&self, state: &S) {
        self.publisher.send(Arc::new(state.clone()));
    }

    fn has_receivers(&self) -> bool {
        self.publisher.has_receivers()
    }
}

/// Sole owner of a [`Store`]: drains the channel and dispatches each event
/// serially, lock-free, so no other task ever touches the state.
///
/// Created by [`init_actor_loop`]; driven by [`run`](ActorLoop::run). The `P`
/// parameter selects the publication policy, [`NoPublish`] by default.
pub struct ActorLoop<S, E, P: Publish<S> = NoPublish> {
    store: Store<S, E>,
    receiver: Receiver<E>,
    publisher: P,
}

impl<S, E, P: Publish<S>> ActorLoop<S, E, P> {
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
            self.store.dispatch(event)?;
            if self.publisher.has_receivers() {
                self.publisher.publish(self.store.state());
            }
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
    let actor_loop = ActorLoop {
        store,
        receiver,
        publisher: NoPublish { _private: () },
    };
    (handle, actor_loop)
}

type PublishingActorLoop<S, E> = ActorLoop<S, E, WatchPublish<S>>;

/// Like [`init_actor_loop`], with a state-change subscription added.
///
/// Also returns a `Stream<Item = Arc<S>>` yielding a state snapshot after each
/// dispatch, once the side-event cascade has settled. The stream replays the
/// current state on subscribe, coalesces intermediate values, and ends when
/// the loop stops.
///
/// `S: Clone` is required only here; [`init_actor_loop`] and the synchronous
/// store stay unaffected. A snapshot is cloned only while a subscriber is
/// alive, so dropping the listener lets the loop skip the clone.
#[must_use = "dropping the returned loop or listener discards the subscription; drive the loop with run()"]
pub fn init_actor_loop_with_subscription<S, E>(
    store: Store<S, E>,
    max_event_buffer: usize,
) -> (
    DispatchHandle<E>,
    PublishingActorLoop<S, E>,
    impl Stream<Item = Arc<S>> + Clone,
)
where
    S: Clone,
{
    let (sender, receiver) = futures::channel::mpsc::channel(max_event_buffer);
    let handle = DispatchHandle { sender };
    let (publisher, listener) = watch::watch(Arc::new(store.state().clone()));
    let actor_loop = ActorLoop {
        store,
        receiver,
        publisher: WatchPublish { publisher },
    };
    (handle, actor_loop, listener)
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
        use futures::{StreamExt, executor::block_on, join};

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
                current_state: &mut SimpleState,
                event: &Self::Event,
            ) -> ReducerOutput<Event> {
                match event {
                    Append { value } => {
                        current_state.name.push(*value);
                        vec![]
                    }
                    Recursive {} => vec![Recursive {}],
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
                Err(err) => assert_eq!(err, DispatchError::MaxDepthExceeded { depth: 11, max: 10 }),
            }
        }

        #[test]
        fn actor_loop_with_subscription() {
            let store = make_store();
            let (mut handle, actor_loop, mut listener) =
                crate::actor::init_actor_loop_with_subscription(store, 10);
            let (_, result) = block_on(async {
                join!(
                    async {
                        assert_eq!(
                            listener
                                .next()
                                .await
                                .expect("A state should be received")
                                .name,
                            ""
                        );
                        handle
                            .try_dispatch(Append { value: 'a' })
                            .expect("Dispatch should succeed");
                        assert_eq!(
                            listener
                                .next()
                                .await
                                .expect("A state should be received")
                                .name,
                            "a"
                        );
                        handle
                            .try_dispatch(Append { value: 'b' })
                            .expect("Dispatch should succeed");
                        assert_eq!(
                            listener
                                .next()
                                .await
                                .expect("A state should be received")
                                .name,
                            "ab"
                        );
                        handle
                            .try_dispatch(Append { value: 'c' })
                            .expect("Dispatch should succeed");
                        assert_eq!(
                            listener
                                .next()
                                .await
                                .expect("A state should be received")
                                .name,
                            "abc"
                        );
                        drop(handle);
                    },
                    actor_loop.run()
                )
            });
            result.expect("Actor loop should run successfully");
        }
    }
}
