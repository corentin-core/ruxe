//! Reducer traits: update operations on `(&mut state, event)`
//!
//! [`Reducer`] operates on the full state. [`SliceReducer`] operates on an
//! isolated slice and composes with peers via tuple syntax (up to arity 12).

/// The output of a reducer: events to re-dispatch through the store, empty for none.
pub type ReducerOutput<E> = Vec<E>;

/// A `Reducer` updates `state` according to `event`.
///
/// Optionally, a reducer may output side events, which are dispatched by the
/// `crate::Store` once dispatch for the current event finishes.
pub trait Reducer<S> {
    /// The event type this reducer handles.
    type Event;

    /// Update the state.
    ///
    /// Aside from `&mut state`, reducers must be pure, i.e., no other mutation,
    /// no I/O.
    fn reduce(&self, state: &mut S, event: &Self::Event) -> ReducerOutput<Self::Event>;
}

/// Like [`Reducer`], but operates on a slice of the global state.
///
/// The `reduce` method receives `&mut Self::Slice`, making access to other
/// slices impossible by construction.
pub trait SliceReducer {
    /// The event type this reducer handles.
    type Event;
    /// The slice of global state this reducer operates on.
    type Slice;

    /// Update the state.
    ///
    /// Aside from `&mut slice`, reducers must be pure, i.e., no other mutation,
    /// no I/O.
    fn reduce(&self, slice: &mut Self::Slice, event: &Self::Event) -> ReducerOutput<Self::Event>;
}
