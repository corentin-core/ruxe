//! An Energy Management System (EMS) built on ruxe — sequential vs parallel.
//!
//! An EMS supervises a power installation. This example models a small plant
//! with three independent subsystems, each held as a state slice:
//!
//! - **Solar** — a PV inverter producing power
//! - **Battery** — stores and releases energy; tracks a state of charge (SOC)
//! - **Power meter** — measures the plant's exchange with the grid
//!
//! Units: active power in watts (W), reactive power in volt-amperes reactive
//! (VAR), voltage in volts (V), state of charge as a percentage (%).
//!
//! ## What it demonstrates
//!
//! - A `PlantState` composed of 3 isolated slices, each with its own
//!   [`SliceReducer`], wrapped in either [`SequentialRootReducer`] or
//!   [`ParallelRootReducer`]
//! - Typed events (an `enum`) rather than stringly-typed actions
//! - Two middlewares: a logger that traces every event and state, and a
//!   battery controller that reacts to a fact by issuing a command
//! - Side events emitted from **both** a reducer and a middleware, then
//!   re-dispatched by the store
//! - A side-by-side timing comparison showing parallel execution speedup
//!
//! ## Simulated work
//!
//! Each slice reducer calls `std::thread::sleep` for 50ms to simulate
//! non-trivial work (e.g. heavy calculation, IO). In production a real
//! reducer would do CPU-bound work; sleep is used here purely to make
//! the wall-clock difference between sequential and parallel observable
//! without writing a contrived computation.
//!
//! ## Scenario (run twice — once per root-reducer variant)
//!
//! Three telemetry events are dispatched (solar, battery, power-meter updates).
//! The battery update drops the SOC to 10%: the battery reducer detects the
//! threshold breach and emits `BatteryStateOfChargeLow` as a side event. The
//! control middleware observes that fact and emits a `BatteryCommand` that
//! zeroes the battery's output — showing reducers and middlewares cooperating
//! through the event queue without ever touching each other's slice.
//!
//! Run with: `cargo run --example ems`

mod common;

use common::*;

use std::time::{Duration, Instant};

use ruxe::{ParallelRootReducer, Reducer, SequentialRootReducer, Store};

fn run_scenario(root_reducer: impl Reducer<PlantState, Event = Event> + 'static) -> Duration {
    let mut store = Store::new(initial_state(), root_reducer, middlewares(), 10);

    let start = Instant::now();

    store
        .dispatch(Event::SolarUpdate {
            active_power: 100.0,
            reactive_power: 50.0,
            voltage: 240.0,
        })
        .expect("dispatch failed");
    store
        .dispatch(Event::BatteryUpdate {
            state_of_charge: 10.0,
            active_power: 50.0,
            reactive_power: 25.0,
        })
        .expect("dispatch failed");
    store
        .dispatch(Event::PowerMeterUpdate {
            active_power: 150.0,
            reactive_power: 75.0,
            voltage: 240.0,
        })
        .expect("dispatch failed");

    let duration = start.elapsed();

    println!("Final state: {}", store.state());

    duration
}

fn main() {
    println!("=== EMS Example — Sequential ===\n");
    let sequential_duration = run_scenario(SequentialRootReducer::new((
        SolarReducer,
        BatteryReducer,
        PowerMeterReducer,
    )));

    println!("\n=== EMS Example — Parallel ===\n");
    let parallel_duration = run_scenario(ParallelRootReducer::new((
        SolarReducer,
        BatteryReducer,
        PowerMeterReducer,
    )));

    println!("\n=== Timing comparison ===");
    println!("Sequential: {:?}", sequential_duration);
    println!("Parallel:   {:?}", parallel_duration);
    println!(
        "Speedup:    {:.2}x",
        sequential_duration.as_secs_f64() / parallel_duration.as_secs_f64()
    );
}
