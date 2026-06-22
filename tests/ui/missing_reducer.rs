//! Compile-fail test: `ParallelRootReducer` must refuse to compile when the
//! state declares a slice that no reducer in the tuple handles.
//!
//! In this test, `MyState::Slices = HList!(FirstSlice, SecondSlice)` but the
//! tuple only contains `FirstReducer` (covering `FirstSlice`). `SecondSlice`
//! is orphan — no reducer matches it. The compiler should report a
//! "trait not implemented" error: no `FindReducerBySlice<SecondSlice, _>`
//! impl exists for the reducer HList.
//!
//! This is one of the two compile-time guarantees of `ParallelRootReducer`:
//! every declared slice must have exactly one matching reducer (bijection).
//!
//! Companion `.stderr` file captures the exact error message — re-bless with
//! `TRYBUILD=overwrite cargo test --test compile_fail` if the message changes.

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

    let state = MyState {
        first: FirstSlice { value: 0 },
        second: SecondSlice { value: 0.0 },
    };
    let event = MyEvent::UpdateFirst(FirstSlice { value: 1 });

    // Only `FirstReducer` is provided, but `MyState` declares both
    // `FirstSlice` and `SecondSlice` in its `StateSlices` impl. When
    // walking `MyState::Slices` to resolve each reducer, the compiler
    // can't find a `FindReducerBySlice<SecondSlice, _>` impl for the
    // HList `HCons<FirstReducer, HNil>` — error at `.reduce(...)`.
    let root_reducer = ParallelRootReducer::new((FirstReducer,));
    let _ = root_reducer.reduce(&state, &event);
}