//! State slice access. Required to compose [`crate::SliceReducer`] over a
//! struct containing multiple slices.

use crate::hlist::HList;

/// Gives access to a state slice for reading and update.
///
/// Implement once per slice type your state contains. The state type holds
/// the slices as fields; this trait exposes them generically so reducers
/// can read and replace them without coupling to the concrete struct.
pub trait HasSlice<T> {
    /// Returns a reference to the slice.
    fn slice(&mut self) -> &mut T;
}

/// Declares the exhaustive list of slices a state contains.
///
/// Required by [`crate::ParallelRootReducer`] to walk the state's slices at
/// compile time and match each one to a reducer. The state must also
/// implement [`HasSlice<T>`] for every `T` in [`Self::Slices`].
///
/// # Why declare it
///
/// Rust's trait system cannot enumerate the `T` for which `HasSlice<T>` is
/// implemented — the user must list them explicitly. A future
/// `#[derive(StateSlices)]` macro will generate this impl from the struct's
/// fields.
///
/// # Ordering
///
/// The order of slices in `Self::Slices` is observable in one place: it
/// determines the order side events are concatenated in
/// [`ParallelRootReducer`](crate::ParallelRootReducer) output. The runtime
/// execution order of slice reducers is not constrained.
///
/// # Example
///
/// ```ignore
/// impl StateSlices for AppState {
///     type Slices = ruxe::HList!(CounterSlice, UserSlice);
/// }
/// ```
pub trait StateSlices {
    /// The HList of slice types this state contains, in declaration order.
    type Slices<'s>: HList
    where
        Self: 's;

    fn to_slices(&mut self) -> Self::Slices<'_>;
}
