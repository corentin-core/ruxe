//! A sequential Energy Management System (EMS) built on ruxe.
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
//!   [`SliceReducer`], combined into a root reducer via tuple syntax
//! - Typed events (an `enum`) rather than stringly-typed actions
//! - Two middlewares: a logger that traces every event and state, and a
//!   battery controller that reacts to a fact by issuing a command
//! - Side events emitted from **both** a reducer and a middleware, then
//!   re-dispatched by the store
//!
//! ## Scenario
//!
//! Three telemetry events are dispatched (solar, battery, power-meter updates).
//! The battery update drops the SOC to 10%: the battery reducer detects the
//! threshold breach and emits `BatteryStateOfChargeLow` as a side event. The
//! control middleware observes that fact and emits a `BatteryCommand` that
//! zeroes the battery's output — showing reducers and middlewares cooperating
//! through the event queue without ever touching each other's slice.
//!
//! Run with: `cargo run --example ems_sequential`
//!
//! Expected output:
//! ```text
//! EMS Sequential Example
//! Event received: SolarUpdate: 100.0W, 50.0VAR, 240.0V
//! Current state: Solar: 0.0W, 0.0VAR, 0.0V, Battery: 100.0%, 0.0W, 0.0VAR, Power Meter: 0.0W, 0.0VAR, 0.0V
//! New state: Solar: 100.0W, 50.0VAR, 240.0V, Battery: 100.0%, 0.0W, 0.0VAR, Power Meter: 0.0W, 0.0VAR, 0.0V
//! Event received: BatteryUpdate: 10.0%, 50.0W, 25.0VAR
//! Current state: Solar: 100.0W, 50.0VAR, 240.0V, Battery: 100.0%, 0.0W, 0.0VAR, Power Meter: 0.0W, 0.0VAR, 0.0V
//! New state: Solar: 100.0W, 50.0VAR, 240.0V, Battery: 10.0%, 50.0W, 25.0VAR, Power Meter: 0.0W, 0.0VAR, 0.0V
//! Event received: BatteryStateOfChargeLow
//! Current state: Solar: 100.0W, 50.0VAR, 240.0V, Battery: 10.0%, 50.0W, 25.0VAR, Power Meter: 0.0W, 0.0VAR, 0.0V
//! Battery state of charge is low. Activating battery control.
//! Issuing command: BatteryCommand: 0.0W, 0.0VAR to reduce battery output.
//! New state: Solar: 100.0W, 50.0VAR, 240.0V, Battery: 10.0%, 50.0W, 25.0VAR, Power Meter: 0.0W, 0.0VAR, 0.0V
//! Event received: BatteryCommand: 0.0W, 0.0VAR
//! Current state: Solar: 100.0W, 50.0VAR, 240.0V, Battery: 10.0%, 50.0W, 25.0VAR, Power Meter: 0.0W, 0.0VAR, 0.0V
//! New state: Solar: 100.0W, 50.0VAR, 240.0V, Battery: 10.0%, 0.0W, 0.0VAR, Power Meter: 0.0W, 0.0VAR, 0.0V
//! Event received: PowerMeterUpdate: 150.0W, 75.0VAR, 240.0V
//! Current state: Solar: 100.0W, 50.0VAR, 240.0V, Battery: 10.0%, 0.0W, 0.0VAR, Power Meter: 0.0W, 0.0VAR, 0.0V
//! New state: Solar: 100.0W, 50.0VAR, 240.0V, Battery: 10.0%, 0.0W, 0.0VAR, Power Meter: 150.0W, 75.0VAR, 240.0V
//! Final state: Solar: 100.0W, 50.0VAR, 240.0V, Battery: 10.0%, 0.0W, 0.0VAR, Power Meter: 150.0W, 75.0VAR, 240.0V
//! ```

use std::fmt::Display;

use ruxe::{HasSlice, Middleware, Next, Reducer, ReducerOutput, SliceReducer, Store};

#[derive(Clone)]
struct SolarState {
    active_power: f64,
    reactive_power: f64,
    voltage: f64,
}

impl Display for SolarState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:.1}W, {:.1}VAR, {:.1}V",
            self.active_power, self.reactive_power, self.voltage
        )
    }
}

#[derive(Clone)]
struct BatteryState {
    state_of_charge: f64,
    active_power: f64,
    reactive_power: f64,
}

impl Display for BatteryState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:.1}%, {:.1}W, {:.1}VAR",
            self.state_of_charge, self.active_power, self.reactive_power
        )
    }
}

#[derive(Clone)]
struct PowerMeterState {
    active_power: f64,
    reactive_power: f64,
    voltage: f64,
}

impl Display for PowerMeterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:.1}W, {:.1}VAR, {:.1}V",
            self.active_power, self.reactive_power, self.voltage
        )
    }
}

#[derive(Clone)]
struct PlantState {
    solar: SolarState,
    battery: BatteryState,
    power_meter: PowerMeterState,
}

impl Display for PlantState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Solar: {}, Battery: {}, Power Meter: {}",
            self.solar, self.battery, self.power_meter
        )
    }
}

impl HasSlice<SolarState> for PlantState {
    fn slice(&self) -> &SolarState {
        &self.solar
    }

    fn set_slice(mut self, slice: SolarState) -> Self {
        self.solar = slice;
        self
    }
}

impl HasSlice<BatteryState> for PlantState {
    fn slice(&self) -> &BatteryState {
        &self.battery
    }

    fn set_slice(mut self, slice: BatteryState) -> Self {
        self.battery = slice;
        self
    }
}

impl HasSlice<PowerMeterState> for PlantState {
    fn slice(&self) -> &PowerMeterState {
        &self.power_meter
    }

    fn set_slice(mut self, slice: PowerMeterState) -> Self {
        self.power_meter = slice;
        self
    }
}

#[derive(Clone)]
enum Event {
    SolarUpdate {
        active_power: f64,
        reactive_power: f64,
        voltage: f64,
    },
    BatteryUpdate {
        state_of_charge: f64,
        active_power: f64,
        reactive_power: f64,
    },
    PowerMeterUpdate {
        active_power: f64,
        reactive_power: f64,
        voltage: f64,
    },
    // Triggered when the battery state of charge falls below a certain threshold
    BatteryStateOfChargeLow,
    // Command to control the battery, e.g., to reduce active power output
    BatteryCommand {
        active_power: f64,
        reactive_power: f64,
    },
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::SolarUpdate {
                active_power,
                reactive_power,
                voltage,
            } => {
                write!(
                    f,
                    "SolarUpdate: {:.1}W, {:.1}VAR, {:.1}V",
                    active_power, reactive_power, voltage
                )
            }
            Event::BatteryUpdate {
                state_of_charge,
                active_power,
                reactive_power,
            } => {
                write!(
                    f,
                    "BatteryUpdate: {:.1}%, {:.1}W, {:.1}VAR",
                    state_of_charge, active_power, reactive_power
                )
            }
            Event::PowerMeterUpdate {
                active_power,
                reactive_power,
                voltage,
            } => {
                write!(
                    f,
                    "PowerMeterUpdate: {:.1}W, {:.1}VAR, {:.1}V",
                    active_power, reactive_power, voltage
                )
            }
            Event::BatteryStateOfChargeLow => write!(f, "BatteryStateOfChargeLow"),
            Event::BatteryCommand {
                active_power,
                reactive_power,
            } => {
                write!(
                    f,
                    "BatteryCommand: {:.1}W, {:.1}VAR",
                    active_power, reactive_power
                )
            }
        }
    }
}

struct LoggingMiddleware;

impl Middleware<PlantState, Event> for LoggingMiddleware {
    fn wrap(self: Box<Self>, mut next: Next<PlantState, Event>) -> Next<PlantState, Event> {
        Box::new(move |state, event| {
            println!("Event received: {}", event);
            println!("Current state: {}", state);
            let side_events = next(state, event);
            println!("New state: {}", state);
            side_events
        })
    }
}

struct BatteryControlMiddleware;

impl Middleware<PlantState, Event> for BatteryControlMiddleware {
    fn wrap(self: Box<Self>, mut next: Next<PlantState, Event>) -> Next<PlantState, Event> {
        Box::new(move |state, event| {
            if let Event::BatteryStateOfChargeLow = event {
                println!("Battery state of charge is low. Activating battery control.");
                let command = Event::BatteryCommand {
                    active_power: 0.0,
                    reactive_power: 0.0,
                };
                println!("Issuing command: {} to reduce battery output.", command);
                let mut side_events = next(state, event).unwrap_or_default();
                side_events.push(command);
                Some(side_events)
            } else {
                next(state, event)
            }
        })
    }
}

struct SolarReducer;

impl SliceReducer for SolarReducer {
    type Event = Event;
    type Slice = SolarState;

    fn reduce(
        &self,
        state: &SolarState,
        event: &Self::Event,
    ) -> ReducerOutput<SolarState, Self::Event> {
        match event {
            Event::SolarUpdate {
                active_power,
                reactive_power,
                voltage,
            } => ReducerOutput {
                state: SolarState {
                    active_power: *active_power,
                    reactive_power: *reactive_power,
                    voltage: *voltage,
                },
                side_events: None,
            },
            _ => ReducerOutput {
                state: state.clone(),
                side_events: None,
            },
        }
    }
}

struct BatteryReducer;

impl SliceReducer for BatteryReducer {
    type Event = Event;
    type Slice = BatteryState;

    fn reduce(
        &self,
        state: &BatteryState,
        event: &Self::Event,
    ) -> ReducerOutput<BatteryState, Self::Event> {
        match event {
            Event::BatteryUpdate {
                state_of_charge,
                active_power,
                reactive_power,
            } => {
                let new_state = BatteryState {
                    state_of_charge: *state_of_charge,
                    active_power: *active_power,
                    reactive_power: *reactive_power,
                };

                let mut side_events = None;
                if new_state.state_of_charge < 20.0 {
                    side_events = Some(vec![Event::BatteryStateOfChargeLow]);
                }
                ReducerOutput {
                    state: new_state,
                    side_events,
                }
            }
            Event::BatteryCommand {
                active_power,
                reactive_power,
            } => ReducerOutput {
                state: BatteryState {
                    state_of_charge: state.state_of_charge,
                    active_power: *active_power,
                    reactive_power: *reactive_power,
                },
                side_events: None,
            },
            _ => ReducerOutput {
                state: state.clone(),
                side_events: None,
            },
        }
    }
}

struct PowerMeterReducer;

impl SliceReducer for PowerMeterReducer {
    type Event = Event;
    type Slice = PowerMeterState;
    fn reduce(
        &self,
        state: &PowerMeterState,
        event: &Self::Event,
    ) -> ReducerOutput<PowerMeterState, Self::Event> {
        match event {
            Event::PowerMeterUpdate {
                active_power,
                reactive_power,
                voltage,
            } => ReducerOutput {
                state: PowerMeterState {
                    active_power: *active_power,
                    reactive_power: *reactive_power,
                    voltage: *voltage,
                },
                side_events: None,
            },
            _ => ReducerOutput {
                state: state.clone(),
                side_events: None,
            },
        }
    }
}

fn make_root_reducer() -> impl Reducer<PlantState, Event = Event> {
    (SolarReducer, BatteryReducer, PowerMeterReducer)
}

fn main() {
    println!("EMS Sequential Example");

    let initial_state = PlantState {
        solar: SolarState {
            active_power: 0.0,
            reactive_power: 0.0,
            voltage: 0.0,
        },
        battery: BatteryState {
            state_of_charge: 100.0,
            active_power: 0.0,
            reactive_power: 0.0,
        },
        power_meter: PowerMeterState {
            active_power: 0.0,
            reactive_power: 0.0,
            voltage: 0.0,
        },
    };
    let root_reducer = make_root_reducer();
    let middlewares: Vec<Box<dyn Middleware<PlantState, Event>>> = vec![
        Box::new(LoggingMiddleware),
        Box::new(BatteryControlMiddleware),
    ];
    let mut store = Store::new(initial_state, root_reducer, middlewares, 10);

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

    println!("Final state: {}", store.state());
}
