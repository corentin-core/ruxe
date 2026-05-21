//! Redux-inspired state management for Rust.
//!
//! A [`Store`] dispatches events through a [`Middleware`] chain to a
//! [`Reducer`], which produces a new state. Reducers can target the full
//! state or an isolated slice via [`SliceReducer`] (composed with tuple
//! syntax). Side events emitted anywhere are re-dispatched in FIFO order.

mod middleware;
mod reducer;
mod state;
mod store;

pub use middleware::Middleware;
pub use middleware::Next;
pub use reducer::Reducer;
pub use reducer::ReducerOutput;
pub use reducer::SliceReducer;
pub use state::HasSlice;
pub use store::DispatchError;
pub use store::Store;
