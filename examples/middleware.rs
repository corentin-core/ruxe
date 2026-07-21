//! Demonstrates wrapping a Store with a custom logging middleware.
//!
//! Run with: `cargo run --example middleware`
//!
//! Expected output:
//! ```text
//! initial state: State(0)
//! dispatching increment
//! next state State(1)
//! dispatching decrement
//! next state State(0)
//! ```

use ruxe::{Middleware, Next, Reducer, ReducerOutput, Store};
use std::fmt;
use std::fmt::Display;

enum Event {
    Increment {},
    Decrement {},
}

impl Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::Increment {} => f.write_str("increment"),
            Event::Decrement {} => f.write_str("decrement"),
        }
    }
}

struct State {
    value: i32,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "State({})", self.value)
    }
}

struct CounterReducer;

impl Reducer<State> for CounterReducer {
    type Event = Event;

    fn reduce(&self, state: &mut State, event: &Event) -> ReducerOutput<Event> {
        match event {
            Event::Increment {} => {
                state.value += 1;
                vec![]
            }
            Event::Decrement {} => {
                state.value -= 1;
                vec![]
            }
        }
    }
}

struct LoggingMiddleware;

impl Middleware<State, Event> for LoggingMiddleware {
    fn wrap(self: Box<Self>, mut next: Next<State, Event>) -> Next<State, Event> {
        Box::new(move |state, event| {
            println!("dispatching {event}");
            let output = next(state, event);
            println!("next state {state}");
            output
        })
    }
}
fn main() {
    let state = State { value: 0 };
    println!("initial state: {state}");
    let mut store = Store::new(state, CounterReducer, vec![Box::new(LoggingMiddleware)], 5);

    store
        .dispatch(Event::Increment {})
        .expect("Failed to increment value");

    store
        .dispatch(Event::Decrement {})
        .expect("Failed to decrement value");
}
