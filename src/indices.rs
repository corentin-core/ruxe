//! Type-level positional witnesses used to disambiguate impls of recursive
//! HList traits.
//!
//! These types encode the position of an element in an HList as a Peano
//! numeral (`Here` = 0, `There<Here>` = 1, `There<There<Here>>` = 2, …).
//! The compiler infers them automatically during trait resolution — they
//! are never written by the user, but appear in the concrete types of
//! values like `ParallelRootReducer<L, E, Indices>`.

use std::marker::PhantomData;

/// Position witness: "target is at the head of the current HCons".
///
/// Equivalent to Peano zero.
#[doc(hidden)]
pub struct Here;

/// Position witness: "target is somewhere inside the tail of the current HCons".
///
/// Wraps the inner position `I`, equivalent to "Peano successor of `I`".
/// A path like `There<There<Here>>` means "skip two heads, then here".
#[doc(hidden)]
pub struct There<I> {
    _marker: PhantomData<I>,
}
