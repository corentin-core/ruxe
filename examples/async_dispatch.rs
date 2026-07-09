//! Async event ingestion — concurrent producers feeding one actor-driven store.
//!
//! Reuses the EMS domain from `common` (see `examples/ems.rs` for the domain
//! walkthrough) and demonstrates the async dispatch primitive:
//!
//! - `init_actor_loop` wraps the `Store` in an actor loop — the sole owner
//!   of the state
//! - Three tokio tasks dispatch device updates concurrently through cloned,
//!   `Send` `DispatchHandle`s; the loop serializes them into one ordered
//!   stream of dispatches, lock-free
//! - Shutdown is cooperative: producers finish and drop their handles, the
//!   loop drains the channel, then `run()` returns the final `Store`
//!
//! Caveat: the demo reducers block 50ms per event (`REDUCER_WORK_SIMULATION`),
//! and that work runs *inside the actor loop*, on the tokio task awaiting
//! `run()` — acceptable for a demo, an antipattern in real async code.
//!
//! Run with: `cargo run --example async_dispatch`

mod common;

use ruxe::{ParallelRootReducer, Store, init_actor_loop};

use crate::common::*;

#[tokio::main]
async fn main() {
    let root_reducer = ParallelRootReducer::new((SolarReducer, BatteryReducer, PowerMeterReducer));
    let store = Store::new(initial_state(), root_reducer, middlewares(), 10);
    let (handle, actor_loop) = init_actor_loop(store, 10);

    let mut solar_handle = handle.clone();
    // Create tokio tasks sending device state updates periodically to the store. The store will process these updates concurrently.
    let solar_task = tokio::spawn(async move {
        for i in 0..10 {
            solar_handle
                .dispatch(Event::SolarUpdate {
                    active_power: 100.0 + i as f64,
                    reactive_power: 50.0 + i as f64,
                    voltage: 240.0 + i as f64,
                })
                .await
                .expect("Dispatch should succeed");
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    let mut power_meter_handle = handle.clone();
    let power_meter_task = tokio::spawn(async move {
        for i in 0..10 {
            power_meter_handle
                .dispatch(Event::PowerMeterUpdate {
                    active_power: 150.0 + i as f64,
                    reactive_power: 75.0 + i as f64,
                    voltage: 240.0 + i as f64,
                })
                .await
                .expect("Dispatch should succeed");
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        }
    });

    let mut battery_handle = handle.clone();
    let battery_task = tokio::spawn(async move {
        for i in 0..10 {
            battery_handle
                .dispatch(Event::BatteryUpdate {
                    state_of_charge: 50.0 - (i as f64 * 5.0),
                    active_power: 50.0 + i as f64,
                    reactive_power: 25.0 + i as f64,
                })
                .await
                .expect("Dispatch should succeed");
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
    });

    drop(handle); // Close the sender to stop the actor loop after processing the events
    let (_, _, _, result) =
        tokio::join!(solar_task, power_meter_task, battery_task, actor_loop.run());
    let store = result.expect("Actor loop should complete successfully");
    println!("Final state: {}", store.state());
}
