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

/// A typed state container that dispatches events through a reducer.
///
/// The store owns the state and the reducer. State can only be modified
/// by dispatching events, ensuring a predictable unidirectional data flow.
pub struct Store<S, R>
{
    state: S,
    reducer: R,
}

impl<S, R : Reducer<S>> Store<S, R>
{
    /// Creates a new store with the given initial state and reducer.
    pub fn new(state: S, reducer: R) -> Self {
        Store{state, reducer}
    }

    /// Dispatches an event through the reducer, updating the state.
    pub fn dispatch(&mut self, event: R::Event) {
        let new_state = self.reducer.reduce(&self.state, event);
        self.state = new_state;
    }

    /// Returns a reference to the current state.
    pub fn state(&self) -> &S {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use crate::{Reducer, Store};
    use crate::tests::Event::{FirstValueUpdate, OtherUpdate, SecondValueUpdate, ThirdValueUpdate};

    enum Event {
        FirstValueUpdate { value: f64 },
        SecondValueUpdate { value: f64 },
        ThirdValueUpdate { value: u32 },
        OtherUpdate {},
    }

    #[derive(Clone, Debug, PartialEq)]
    struct SimpleState {
        first_value: f64,
        second_value: f64,
        third_value: u32
    }

    struct SimpleReducer
    {}

    impl Reducer<SimpleState> for SimpleReducer {
        type Event = Event;

        fn reduce(&self, state: &SimpleState, event: Self::Event) -> SimpleState {
            match event {
                FirstValueUpdate {value} => {
                    SimpleState{first_value: value, ..*state }
                },
                SecondValueUpdate {value} => {
                    SimpleState{second_value: value, ..*state}
                },
                ThirdValueUpdate {value} => {
                    SimpleState{third_value: value, ..*state}
                },
                _ => { state.clone() }
            }
        }
    }

    fn make_state() -> SimpleState
    {
        SimpleState{
            first_value: 1.5,
            second_value: 1.6,
            third_value: 3
        }
    }

    fn make_store() -> Store<SimpleState, SimpleReducer>
    {
        let reducer = SimpleReducer{};
        let state = make_state();
        Store::new(state.clone(), reducer)
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

        store.dispatch(FirstValueUpdate { value: 1.2 });
        assert_eq!(*store.state(), SimpleState {
            first_value: 1.2,
            ..state
        });
    }

    #[test]
    fn multiple_dispatch() {
        let mut store = make_store();
        let state = make_state();

        store.dispatch(FirstValueUpdate {value: 1.2});
        assert_eq!(*store.state(), SimpleState{
            first_value: 1.2,
            ..state
        });

        let state = store.state().clone();
        store.dispatch(SecondValueUpdate {value: 1.3});
        assert_eq!(*store.state(), SimpleState{
            second_value: 1.3,
            ..state
        });

        let state = store.state().clone();
        store.dispatch(ThirdValueUpdate {value: 5});
        assert_eq!(*store.state(), SimpleState{
            third_value: 5,
            ..state
        });
    }

    #[test]
    fn unrelated_dispatch() {
        let mut store = make_store();
        let state = make_state();

        store.dispatch(OtherUpdate {});
        assert_eq!(*store.state(), state);
    }
}