/// A reducer takes the current state and an event, and returns a new state.
///
/// The reducer does not modify the state in place — it produces a new value
/// that the [`Store`] will use to replace the current state.
pub trait Reducer<S> {
    /// The event type this reducer handles.
    type Event;

    /// Computes a new state from the current state and an event.
    fn reduce(&self, state: &S, event: Self::Event) -> S;
}
