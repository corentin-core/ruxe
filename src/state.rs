/// Gives access to a state slice for reading and update.
pub trait HasSlice<T> {
    /// Returns a reference to the slice.
    fn slice(&self) -> &T;
    /// Replaces the slice with a new value, returning the updated state.
    fn set_slice(self, slice: T) -> Self;
}
