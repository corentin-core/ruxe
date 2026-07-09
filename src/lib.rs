//! Redux-inspired state management for Rust.
//!
//! A [`Store`] dispatches events through a [`Middleware`] chain to a
//! [`Reducer`], which produces a new state. Reducers can target the full
//! state or an isolated slice via [`SliceReducer`] (composed with tuple
//! syntax). Side events emitted anywhere are re-dispatched in FIFO order.

#[cfg(test)]
mod fixtures;

pub mod actor;
mod hlist;
mod indices;
mod middleware;
mod parallel_root_reducer;
mod reducer;
mod sequential_root_reducer;
mod state;
mod store;

pub use actor::{ActorLoop, DispatchHandle, init_actor_loop};
#[doc(hidden)]
pub use hlist::{HCons, HList, HNil, IntoHList};
#[doc(hidden)]
pub use indices::{Here, There};
pub use middleware::Middleware;
pub use middleware::Next;
pub use parallel_root_reducer::ParallelRootReducer;
pub use reducer::Reducer;
pub use reducer::ReducerOutput;
pub use reducer::SliceReducer;
pub use sequential_root_reducer::SequentialRootReducer;
pub use state::HasSlice;
pub use state::StateSlices;
pub use store::DispatchError;
pub use store::Store;
