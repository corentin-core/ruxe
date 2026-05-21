//! Middleware chain composition for the dispatch pipeline.
//!
//! Middlewares wrap the dispatch chain to add cross-cutting concerns
//! (logging, metrics, validation, conditional dispatch). They follow the
//! onion model: the first middleware in the list is the outermost — it
//! sees the event first and the resulting state last.
//!
//! ```text
//! dispatch(event)
//!     │
//!     ▼
//!  ┌─ M1 ───────────────────────────────┐
//!  │ pre1                               │
//!  │  ┌─ M2 ───────────────────────────┐│
//!  │  │ pre2                           ││
//!  │  │  ┌─ reducer ─────────────────┐ ││
//!  │  │  │ state mutated in place    │ ││  ← commit
//!  │  │  └───────────────────────────┘ ││
//!  │  │ post2  (sees new state)        ││
//!  │  └────────────────────────────────┘│
//!  │ post1  (sees new state)            │
//!  └────────────────────────────────────┘
//! ```

/// A node in the dispatch chain.
///
/// Mutates state in place and returns any side events. Code running after
/// `next` returns observes the updated state.
pub type Next<S, E> = Box<dyn FnMut(&mut S, E) -> Option<Vec<E>>>;

/// A wrapper around the dispatch chain.
///
/// [`wrap`](Middleware::wrap) returns a closure that calls `next` zero or
/// one time:
///
/// - **Calling `next`** runs the inner chain.
/// - **Skipping `next`** short-circuits: state unchanged, side events come
///   only from this middleware.
///
/// Registered with [`crate::Store::new`].
pub trait Middleware<S, E> {
    /// Composes with the inner chain `next`. The boxed `self` lets owned
    /// state move into the returned closure.
    fn wrap(self: Box<Self>, next: Next<S, E>) -> Next<S, E>;
}
