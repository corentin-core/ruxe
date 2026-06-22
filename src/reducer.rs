//! Reducer traits: pure transformations from `(state, event)` to new state.
//!
//! [`Reducer`] operates on the full state. [`SliceReducer`] operates on an
//! isolated slice and composes with peers via tuple syntax (up to arity 12).

/// The output of a reducer.
pub struct ReducerOutput<S, E> {
    /// The modified state.
    pub state: S,
    /// Optional events to re-dispatch through the store.
    pub side_events: Option<Vec<E>>,
}

/// A pure transformation: `(state, event)` → new state + optional side events.
///
/// The [`crate::Store`] replaces its state with the returned value; the
/// input `&S` is not modified.
pub trait Reducer<S> {
    /// The event type this reducer handles.
    type Event;

    /// Computes the new state. Must be pure: no I/O, no mutation.
    fn reduce(&self, state: &S, event: &Self::Event) -> ReducerOutput<S, Self::Event>;
}

/// Like [`Reducer`], but operates on a slice of the global state.
///
/// The `reduce` method receives `&Self::Slice` only, making access to other
/// slices impossible by construction.
pub trait SliceReducer {
    /// The event type this reducer handles.
    type Event;
    /// The slice of global state this reducer operates on.
    type Slice;

    /// Computes the new slice. Must be pure: no I/O, no mutation.
    fn reduce(
        &self,
        slice: &Self::Slice,
        event: &Self::Event,
    ) -> ReducerOutput<Self::Slice, Self::Event>;
}
