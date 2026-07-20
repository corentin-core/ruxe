//! Async event ingestion: concurrent producers feeding one actor-driven store,
//! plus a controller reacting to state changes.
//!
//! Reuses the EMS domain from `common` (see `examples/ems.rs` for the domain
//! walkthrough) and demonstrates the async dispatch and subscription primitives:
//!
//! - `init_actor_loop_with_subscription` wraps the `Store` in an actor loop
//!   (the sole owner of the state) and returns a state `Stream`
//! - Three tokio tasks dispatch device updates concurrently through cloned,
//!   `Send` `DispatchHandle`s; the loop serializes them lock-free
//! - A `SolarController` task reacts to state changes from the `Stream` and
//!   dispatches commands back through its own handle, closing the
//!   react-to-state loop
//! - Shutdown is cooperative: a `Termination` event flips a state flag the
//!   controller observes, so it stops and drops its handle. Once every handle
//!   is gone, the loop drains the channel and `run()` returns the final `Store`
//!
//! Caveat: the demo reducers block 50ms per event (`REDUCER_WORK_SIMULATION`),
//! and that work runs *inside the actor loop*, on the tokio task awaiting
//! `run()`. Fine for a demo, an antipattern in real async code.
//!
//! Run with: `cargo run --example async_dispatch`

mod common;

use crate::common::*;
use futures::Stream;
use futures::StreamExt;
use ruxe::{DispatchHandle, ParallelRootReducer, Store, init_actor_loop_with_subscription};
use std::sync::Arc;

struct SolarController<Listener: Stream<Item = Arc<PlantState>> + Unpin> {
    dispatch_handle: DispatchHandle<Event>,
    state_listener: Listener,
}

impl<Listener> SolarController<Listener>
where
    Listener: Stream<Item = Arc<PlantState>> + Unpin,
{
    async fn run(mut self) {
        while let Some(state) = self.state_listener.next().await {
            if state.system.termination_requested {
                println!("Termination requested. Stopping SolarController.");
                break;
            }

            if state.battery.active_power == 0.0 && state.solar.active_power > 0.0 {
                println!("Battery stopped, stopping solar power generation.");
                if self
                    .dispatch_handle
                    .dispatch(Event::SolarUpdate {
                        active_power: 0.0,
                        reactive_power: 0.0,
                        voltage: 0.0,
                    })
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let root_reducer = ParallelRootReducer::new((
        SystemReducer,
        SolarReducer,
        BatteryReducer,
        PowerMeterReducer,
    ));
    let store = Store::new(initial_state(), root_reducer, middlewares(), 10);
    let (handle, actor_loop, listener) = init_actor_loop_with_subscription(store, 10);

    let solar_controller = SolarController {
        dispatch_handle: handle.clone(),
        state_listener: listener,
    };

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

    let mut termination_handle = handle.clone();
    let termination_task = tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        termination_handle
            .dispatch(Event::Termination {})
            .await
            .expect("Termination signal should be sent");
    });

    drop(handle); // Close the sender to stop the actor loop after processing the events
    let (_, _, _, _, _, result) = tokio::join!(
        termination_task,
        solar_controller.run(),
        solar_task,
        power_meter_task,
        battery_task,
        actor_loop.run()
    );
    let store = result.expect("Actor loop should complete successfully");
    println!("Final state: {}", store.state());
}
