#![allow(dead_code)]

use std::{fmt::Display, thread, time::Duration};

use ruxe::{HasSlice, Middleware, Next, ReducerOutput, SliceReducer, StateSlices};

/// Simulated per-event work: every demo reducer blocks this long
/// (`thread::sleep`), standing in for real CPU/IO cost. Each example's
/// header states what this implies for it.
const REDUCER_WORK_SIMULATION: Duration = Duration::from_millis(50);

#[derive(Clone)]
pub struct SolarState {
    pub active_power: f64,
    pub reactive_power: f64,
    pub voltage: f64,
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
pub struct BatteryState {
    pub state_of_charge: f64,
    pub active_power: f64,
    pub reactive_power: f64,
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
pub struct PowerMeterState {
    pub active_power: f64,
    pub reactive_power: f64,
    pub voltage: f64,
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
pub struct System {
    pub termination_requested: bool,
}

impl Display for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Termination requested: {}", self.termination_requested)
    }
}

#[derive(Clone)]
pub struct PlantState {
    pub solar: SolarState,
    pub battery: BatteryState,
    pub power_meter: PowerMeterState,
    pub system: System,
}

impl Display for PlantState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Solar: {}, Battery: {}, Power Meter: {}, System: {}",
            self.solar, self.battery, self.power_meter, self.system
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

impl HasSlice<System> for PlantState {
    fn slice(&self) -> &System {
        &self.system
    }

    fn set_slice(mut self, slice: System) -> Self {
        self.system = slice;
        self
    }
}

impl StateSlices for PlantState {
    type Slices = ruxe::HList!(SolarState, BatteryState, PowerMeterState, System);
}

#[derive(Clone, Debug)]
pub enum Event {
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
    BatteryStateOfChargeLow,
    BatteryCommand {
        active_power: f64,
        reactive_power: f64,
    },
    Termination {},
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::SolarUpdate {
                active_power,
                reactive_power,
                voltage,
            } => write!(
                f,
                "SolarUpdate: {:.1}W, {:.1}VAR, {:.1}V",
                active_power, reactive_power, voltage
            ),
            Event::BatteryUpdate {
                state_of_charge,
                active_power,
                reactive_power,
            } => write!(
                f,
                "BatteryUpdate: {:.1}%, {:.1}W, {:.1}VAR",
                state_of_charge, active_power, reactive_power
            ),
            Event::PowerMeterUpdate {
                active_power,
                reactive_power,
                voltage,
            } => write!(
                f,
                "PowerMeterUpdate: {:.1}W, {:.1}VAR, {:.1}V",
                active_power, reactive_power, voltage
            ),
            Event::BatteryStateOfChargeLow => write!(f, "BatteryStateOfChargeLow"),
            Event::BatteryCommand {
                active_power,
                reactive_power,
            } => write!(
                f,
                "BatteryCommand: {:.1}W, {:.1}VAR",
                active_power, reactive_power
            ),
            Event::Termination {} => write!(f, "Termination"),
        }
    }
}

pub struct LoggingMiddleware;

impl Middleware<PlantState, Event> for LoggingMiddleware {
    fn wrap(self: Box<Self>, mut next: Next<PlantState, Event>) -> Next<PlantState, Event> {
        Box::new(move |state, event| {
            println!("Event received: {}", event);
            println!("Current state: {}", state);
            let side_events = next(state, event);
            println!("New state:     {}", state);
            side_events
        })
    }
}

pub struct BatteryControlMiddleware;

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

pub struct SolarReducer;

impl SliceReducer for SolarReducer {
    type Event = Event;
    type Slice = SolarState;

    fn reduce(
        &self,
        state: &SolarState,
        event: &Self::Event,
    ) -> ReducerOutput<SolarState, Self::Event> {
        thread::sleep(REDUCER_WORK_SIMULATION);
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

pub struct BatteryReducer;

impl SliceReducer for BatteryReducer {
    type Event = Event;
    type Slice = BatteryState;

    fn reduce(
        &self,
        state: &BatteryState,
        event: &Self::Event,
    ) -> ReducerOutput<BatteryState, Self::Event> {
        thread::sleep(REDUCER_WORK_SIMULATION);
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

pub struct PowerMeterReducer;

impl SliceReducer for PowerMeterReducer {
    type Event = Event;
    type Slice = PowerMeterState;

    fn reduce(
        &self,
        state: &PowerMeterState,
        event: &Self::Event,
    ) -> ReducerOutput<PowerMeterState, Self::Event> {
        thread::sleep(REDUCER_WORK_SIMULATION);
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

pub struct SystemReducer;

impl SliceReducer for SystemReducer {
    type Event = Event;
    type Slice = System;

    fn reduce(&self, state: &System, event: &Self::Event) -> ReducerOutput<System, Self::Event> {
        thread::sleep(REDUCER_WORK_SIMULATION);
        match event {
            Event::Termination {} => ReducerOutput {
                state: System {
                    termination_requested: true,
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

pub fn initial_state() -> PlantState {
    PlantState {
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
        system: System {
            termination_requested: false,
        },
    }
}

pub fn middlewares() -> Vec<Box<dyn Middleware<PlantState, Event>>> {
    vec![
        Box::new(LoggingMiddleware),
        Box::new(BatteryControlMiddleware),
    ]
}
