//! Compile-fail test: `ParallelRootReducer` must refuse to compile when two
//! slice reducers target the same `Slice` type.
//!
//! In this test, `FirstReducer` is passed twice in the tuple. Both have
//! `Slice = FirstSlice`, which would create a data race in parallel
//! execution if it were allowed. The compiler should report an
//! "ambiguous impl" error during the resolution of `FindReducerBySlice`
//! (the trait used internally to map state slices to reducers).
//!
//! Companion `.stderr` file captures the exact error message — if you
//! change `FindReducerBySlice` or related machinery, re-bless with
//! `TRYBUILD=overwrite cargo test --test compile_fail`.

use ruxe::{HasSlice, ParallelRootReducer, Reducer, ReducerOutput, SliceReducer, StateSlices};

fn main() {
    #[derive(Clone)]
    struct FirstSlice {
        value: u32,
    }

    #[derive(Clone)]
    struct SecondSlice {
        value: f64,
    }


    #[derive(Clone)]
    struct MyState {
        first: FirstSlice,
        second: SecondSlice,
    }

    impl HasSlice<FirstSlice> for MyState {
        fn slice(&self) -> &FirstSlice {
            &self.first
        }
        fn set_slice(mut self, slice: FirstSlice) -> Self {
            self.first = slice;
            self
        }
    }

    impl HasSlice<SecondSlice> for MyState {
        fn slice(&self) -> &SecondSlice {
            &self.second
        }
        fn set_slice(mut self, slice: SecondSlice) -> Self {
            self.second = slice;
            self
        }
    }

    impl StateSlices for MyState {
        type Slices = ruxe::HList!(FirstSlice, SecondSlice);
    }

    enum MyEvent {
        UpdateFirst(FirstSlice),
        UpdateSecond(SecondSlice),
    }

    struct FirstReducer;

    impl SliceReducer for FirstReducer {
        type Event = MyEvent;
        type Slice = FirstSlice;

        fn reduce(&self, slice: &FirstSlice, event: &MyEvent) -> ReducerOutput<FirstSlice, MyEvent> {
            match event {
                MyEvent::UpdateFirst(v) => ReducerOutput { state: FirstSlice { value: v.value }, side_events: None },
                _ => ReducerOutput { state: slice.clone(), side_events: None },
            }
        }
    }

    struct SecondReducer;

    impl SliceReducer for SecondReducer {
        type Event = MyEvent;
        type Slice = SecondSlice;

        fn reduce(&self, slice: &SecondSlice, event: &MyEvent) -> ReducerOutput<SecondSlice, MyEvent> {
            match event {
                MyEvent::UpdateSecond(v) => ReducerOutput { state: SecondSlice { value: v.value }, side_events: None },
                _ => ReducerOutput { state: slice.clone(), side_events: None },
            }
        }
    }


    let state = MyState {
        first: FirstSlice { value: 0 },
        second: SecondSlice { value: 0.0 },
    };
    let event = MyEvent::UpdateFirst(FirstSlice { value: 1 });

    // Two `FirstReducer` instances target the same `FirstSlice`. The compiler
    // detects this via ambiguous `FindReducerBySlice<FirstSlice, _>` impls
    // (one at `Here`, one at `There<Here>`) when resolving the bounds of
    // `reduce`. The error appears at the `.reduce(...)` call site below —
    // type inference for `Indices` cannot pick a unique path.
    let root_reducer = ParallelRootReducer::new((FirstReducer, FirstReducer, SecondReducer));
    let _ = root_reducer.reduce(&state, &event);
}