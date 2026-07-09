//! Integration tests exercising the full `Store + RootReducer + Middleware`
//! pipeline. Complements unit tests by validating the cross-component flow.

use futures::executor::block_on;
use futures::join;
use ruxe::{
    HasSlice, Middleware, Next, ParallelRootReducer, ReducerOutput, SequentialRootReducer,
    SliceReducer, StateSlices, Store, init_actor_loop,
};

#[derive(Clone, Debug, PartialEq)]
struct CountSlice {
    value: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct LabelSlice {
    text: String,
}

#[derive(Clone, Debug, PartialEq)]
struct AppState {
    count: CountSlice,
    label: LabelSlice,
}

impl HasSlice<CountSlice> for AppState {
    fn slice(&self) -> &CountSlice {
        &self.count
    }
    fn set_slice(mut self, slice: CountSlice) -> Self {
        self.count = slice;
        self
    }
}

impl HasSlice<LabelSlice> for AppState {
    fn slice(&self) -> &LabelSlice {
        &self.label
    }
    fn set_slice(mut self, slice: LabelSlice) -> Self {
        self.label = slice;
        self
    }
}

impl StateSlices for AppState {
    type Slices = ruxe::HList!(CountSlice, LabelSlice);
}

#[derive(Debug, PartialEq)]
enum Event {
    Increment,
    SetLabel(String),
    Ping,
    Pong,
}

struct CountReducer;
impl SliceReducer for CountReducer {
    type Event = Event;
    type Slice = CountSlice;
    fn reduce(&self, slice: &CountSlice, event: &Event) -> ReducerOutput<CountSlice, Event> {
        match event {
            Event::Increment => ReducerOutput {
                state: CountSlice {
                    value: slice.value + 1,
                },
                side_events: None,
            },
            Event::Ping => ReducerOutput {
                state: slice.clone(),
                side_events: Some(vec![Event::Pong]),
            },
            _ => ReducerOutput {
                state: slice.clone(),
                side_events: None,
            },
        }
    }
}

struct LabelReducer;
impl SliceReducer for LabelReducer {
    type Event = Event;
    type Slice = LabelSlice;
    fn reduce(&self, slice: &LabelSlice, event: &Event) -> ReducerOutput<LabelSlice, Event> {
        match event {
            Event::SetLabel(text) => ReducerOutput {
                state: LabelSlice { text: text.clone() },
                side_events: None,
            },
            _ => ReducerOutput {
                state: slice.clone(),
                side_events: None,
            },
        }
    }
}

fn initial_state() -> AppState {
    AppState {
        count: CountSlice { value: 0 },
        label: LabelSlice {
            text: String::from("init"),
        },
    }
}

/// A middleware that records every event it observes — used to verify the
/// dispatch chain flows through middleware as expected.
struct Recorder {
    log: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl Middleware<AppState, Event> for Recorder {
    fn wrap(self: Box<Self>, mut next: Next<AppState, Event>) -> Next<AppState, Event> {
        let log = self.log.clone();
        Box::new(move |state, event| {
            log.lock().unwrap().push(format!("{:?}", event));
            next(state, event)
        })
    }
}

#[test]
fn sequential_root_reducer_via_store() {
    let mut store = Store::new(
        initial_state(),
        SequentialRootReducer::new((CountReducer, LabelReducer)),
        vec![],
        10,
    );

    store.dispatch(Event::Increment).expect("dispatch");
    store.dispatch(Event::Increment).expect("dispatch");
    store
        .dispatch(Event::SetLabel(String::from("done")))
        .expect("dispatch");

    assert_eq!(store.state().count.value, 2);
    assert_eq!(store.state().label.text, "done");
}

#[test]
fn parallel_root_reducer_via_store() {
    let mut store = Store::new(
        initial_state(),
        ParallelRootReducer::new((CountReducer, LabelReducer)),
        vec![],
        10,
    );

    store.dispatch(Event::Increment).expect("dispatch");
    store.dispatch(Event::Increment).expect("dispatch");
    store
        .dispatch(Event::SetLabel(String::from("done")))
        .expect("dispatch");

    assert_eq!(store.state().count.value, 2);
    assert_eq!(store.state().label.text, "done");
}

#[test]
fn parallel_root_reducer_emits_side_events_through_store() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = Recorder { log: log.clone() };

    let mut store = Store::new(
        initial_state(),
        ParallelRootReducer::new((CountReducer, LabelReducer)),
        vec![Box::new(recorder)],
        10,
    );

    store.dispatch(Event::Ping).expect("dispatch");

    let log = log.lock().unwrap();
    assert_eq!(*log, vec!["Ping", "Pong"]);
}

#[test]
fn actor_loop_integration() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = Recorder { log: log.clone() };

    let store = Store::new(
        initial_state(),
        ParallelRootReducer::new((CountReducer, LabelReducer)),
        vec![Box::new(recorder)],
        10,
    );

    let (handle, actor_loop) = init_actor_loop(store, 10);
    let mut handle1 = handle.clone();
    let dispatch_future_1 = async move {
        handle1
            .dispatch(Event::Ping)
            .await
            .expect("Dispatch should succeed");
    };
    let mut handle2 = handle.clone();
    let dispatch_future_2 = async move {
        handle2
            .dispatch(Event::Increment)
            .await
            .expect("Dispatch should succeed");
    };
    let mut handle3 = handle.clone();
    let dispatch_future_3 = async move {
        handle3
            .dispatch(Event::SetLabel(String::from("done")))
            .await
            .expect("Dispatch should succeed");
    };
    drop(handle); // Drop the original handle to avoid deadlock in the actor loop
    let store = block_on(async {
        let (_, _, _, loop_result) = join!(
            dispatch_future_1,
            dispatch_future_2,
            dispatch_future_3,
            actor_loop.run()
        );
        loop_result.expect("Actor loop should complete successfully")
    });

    let log = log.lock().unwrap();

    // The order of events in the log may vary due to concurrency, so we check for presence rather than order.
    assert!(log.contains(&"Ping".to_string()));
    assert!(log.contains(&"Pong".to_string()));
    assert!(log.contains(&"Increment".to_string()));
    assert!(log.contains(&"SetLabel(\"done\")".to_string()));

    assert_eq!(
        store.state(),
        &AppState {
            count: CountSlice { value: 1 },
            label: LabelSlice {
                text: String::from("done"),
            },
        }
    );
}
