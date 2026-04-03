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
/// that the [`Store`] will use to replace the current state.
pub trait Reducer<S> {
    /// The event type this reducer handles.
    type Event;

    /// Computes a new state and optional side events from the current state and an event.
    fn reduce(&self, state: &S, event: Self::Event) -> ReducerOutput<S, Self::Event>;
}
