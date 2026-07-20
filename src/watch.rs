//! A `watch`-style broadcast of the latest value, hand-recoded on `futures`
//! so the core keeps no runtime dependency.
//!
//! [`watch`] pairs a [`Sender`] with a [`Receiver`].
//!
//! Each `send` overwrites a single shared slot and bumps a version, so a
//! receiver observes the latest value; intermediate values are coalesced.
//!
//! A [`Receiver`] is a `Stream<Item = T>`: it yields the current value on first
//! poll, then one item per change, and ends with `None` once the [`Sender`] is
//! dropped.

use std::{
    collections::HashMap,
    pin::Pin,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    task::{Context, Poll},
};

use futures::{Stream, task::Waker};

struct WakerRegistry {
    wakers: HashMap<u64, Waker>,
    next_waker_id: u64,
}

impl WakerRegistry {
    fn new() -> Self {
        Self {
            wakers: HashMap::new(),
            next_waker_id: 0,
        }
    }

    fn reserve_id(&mut self) -> u64 {
        let id = self.next_waker_id;
        self.next_waker_id += 1;
        id
    }

    fn register(&mut self, reg_id: u64, waker: &Waker) {
        self.wakers.insert(reg_id, waker.clone());
    }

    fn unregister(&mut self, id: u64) {
        self.wakers.remove(&id);
    }

    fn collect_wakers(&self) -> Vec<Waker> {
        self.wakers.values().cloned().collect()
    }
}

pub(crate) struct Shared<T> {
    value: Mutex<T>,
    version: AtomicU64,
    wakers: Mutex<WakerRegistry>,
    closed: AtomicBool,
}

impl<T> Shared<T> {
    fn new(value: T) -> Self {
        Self {
            value: Mutex::new(value),
            version: AtomicU64::new(1), // Start at 1 so that initial value is considered changed
            wakers: Mutex::new(WakerRegistry::new()),
            closed: AtomicBool::new(false),
        }
    }

    fn value(&self) -> MutexGuard<'_, T> {
        self.value.lock().expect("Lock should not be poisoned")
    }

    fn wake_registry(&self) -> MutexGuard<'_, WakerRegistry> {
        self.wakers.lock().expect("Lock should not be poisoned")
    }

    fn register(&self, waker_id: u64, waker: &Waker) {
        self.wake_registry().register(waker_id, waker);
    }

    fn unregister(&self, waker_id: u64) {
        self.wake_registry().unregister(waker_id);
    }

    fn reserve_id(&self) -> u64 {
        self.wake_registry().reserve_id()
    }

    fn version(&self) -> u64 {
        self.version.load(Ordering::SeqCst)
    }

    fn wake_receivers(&self) {
        let wakers = self.wake_registry().collect_wakers();
        for waker in wakers {
            waker.wake_by_ref();
        }
    }
}

/// Receiving half: a `Stream<Item = T>` over the latest value. Cloning yields
/// an independent subscriber that replays the current value on its next poll.
pub(crate) struct Receiver<T> {
    shared: Arc<Shared<T>>,
    last_seen_version: u64,
    waker_id: Option<u64>,
}

impl<T> Receiver<T> {
    pub(crate) fn new(shared: Arc<Shared<T>>) -> Self {
        Self {
            shared,
            last_seen_version: 0,
            waker_id: None,
        }
    }
}

impl<T: Clone> Stream for Receiver<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.waker_id.is_none() {
            let waker_id = this.shared.reserve_id();
            this.waker_id = Some(waker_id);
        }

        let waker_id = this.waker_id.expect("waker id should be set");
        // Register the waker before reading version/closed. The reverse order
        // loses a wakeup if a send lands in between.
        this.shared.register(waker_id, cx.waker());

        let guard = this.shared.value();
        let current_version = this.shared.version();
        if current_version != this.last_seen_version {
            this.last_seen_version = current_version;
            Poll::Ready(Some(guard.clone()))
        } else if this.shared.closed.load(Ordering::SeqCst) {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
            last_seen_version: 0, // Start fresh for the new receiver to get the current value
            waker_id: None,
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        if let Some(waker_id) = self.waker_id.take() {
            self.shared.unregister(waker_id);
        }
    }
}

/// Sending half of a [`watch`] channel. Single producer, not `Clone`.
pub(crate) struct Sender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Sender<T> {
    /// Overwrites the latest value and wakes every receiver. A slow receiver
    /// observes only the most recent value.
    pub(crate) fn send(&self, value: T) {
        {
            let mut shared_value = self.shared.value();
            *shared_value = value;
            self.shared.version.fetch_add(1, Ordering::SeqCst);
        }
        self.shared.wake_receivers();
    }

    /// Whether at least one [`Receiver`] is still alive, so a producer can
    /// skip computing a value nobody would observe.
    pub(crate) fn has_receivers(&self) -> bool {
        Arc::strong_count(&self.shared) > 1
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        self.shared.closed.store(true, Ordering::SeqCst);
        self.shared.wake_receivers();
    }
}

/// Creates a [`Sender`]/[`Receiver`] pair seeded with `initial_value`. The
/// initial value counts as the first version, so a fresh receiver yields it on
/// first poll.
pub(crate) fn watch<T>(initial_value: T) -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Shared::new(initial_value));
    let sender = Sender {
        shared: Arc::clone(&shared),
    };
    let receiver = Receiver::new(shared);
    (sender, receiver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use futures::{executor::block_on, join};
    use std::fmt::Debug;

    fn assert_expected_received<T: PartialEq + Clone + Debug>(
        mut receiver: Receiver<T>,
        expected: T,
    ) {
        block_on(async {
            assert_eq!(receiver.next().await, Some(expected));
        });
    }

    #[test]
    fn watch_initial_value() {
        let (_sender, receiver) = watch(0);
        assert_expected_received(receiver, 0);
    }

    #[test]
    fn watch_changed() {
        let (sender, receiver) = watch(0);

        sender.send(1);
        assert_expected_received(receiver, 1);
    }

    #[test]
    fn has_receiver() {
        let (sender, receiver) = watch(0);
        assert!(sender.has_receivers());
        drop(receiver);
        assert!(!sender.has_receivers());
    }

    #[test]
    fn watch_multiple_receivers() {
        let (sender, mut receiver1) = watch(0);
        let mut receiver2 = receiver1.clone();

        block_on(async {
            join!(
                async {
                    assert_eq!(receiver1.next().await, Some(0));
                    assert_eq!(receiver1.next().await, Some(1));
                },
                async {
                    assert_eq!(receiver2.next().await, Some(0));
                    assert_eq!(receiver2.next().await, Some(1));
                },
                async {
                    sender.send(1);
                },
            );
        })
    }

    #[test]
    fn value_coalescing() {
        let (sender, receiver) = watch(0);

        sender.send(1);
        sender.send(2);
        sender.send(3);

        assert_expected_received(receiver, 3);
    }

    #[test]
    fn wakes_parked_receiver() {
        let (sender, mut receiver) = watch(0);
        block_on(async {
            join!(
                async {
                    assert_eq!(receiver.next().await, Some(0));
                    assert_eq!(receiver.next().await, Some(1));
                },
                async {
                    sender.send(1);
                },
            );
        });
    }

    #[test]
    fn drop_sender_wakes_parked_receiver() {
        let (sender, mut receiver) = watch(0);
        block_on(async {
            join!(
                async {
                    assert_eq!(receiver.next().await, Some(0));
                    assert_eq!(receiver.next().await, None);
                },
                async {
                    drop(sender);
                },
            );
        });
    }

    #[test]
    fn drains_final_value_before_close() {
        let (sender, mut receiver) = watch(0);
        block_on(async {
            join!(
                async {
                    assert_eq!(receiver.next().await, Some(0)); // Initial value
                    // Receiver is parked, then sender sends a value and then drops itself
                    // Receiver is awaken and should receive the new value before seeing None
                    assert_eq!(receiver.next().await, Some(1));
                    assert_eq!(receiver.next().await, None);
                },
                async {
                    sender.send(1);
                    drop(sender);
                },
            );
        });
    }

    #[test]
    fn multithreaded() {
        let (sender, mut receiver) = watch(0);
        let sender_thread = std::thread::spawn(move || {
            sender.send(1);
        });
        block_on(async {
            assert_eq!(receiver.next().await, Some(0));
            assert_eq!(receiver.next().await, Some(1));
        });
        sender_thread.join().unwrap();
    }
}
