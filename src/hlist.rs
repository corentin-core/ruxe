//! Minimal HList (heterogeneous list) machinery — internal to ruxe.
//!
//! Used by [`crate::ParallelRootReducer`] to encode the state's slice list
//! as a recursive type and to walk it via recursive trait impls.
//!
//! Items are re-exported at the crate root as `#[doc(hidden)]` — they are
//! forced-public only because the `HList!` macro expands to them in user
//! code (e.g. `type Slices = HList!(A, B)` becomes `HCons<A, HCons<B, HNil>>`
//! in the user's scope). They are not part of the stable user-facing API.

#[doc(hidden)]
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct HNil;

#[doc(hidden)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct HCons<H, T> {
    pub head: H,
    pub tail: T,
}

#[doc(hidden)]
pub trait HList {}

impl HList for HNil {}
impl<H, T: HList> HList for HCons<H, T> {}

macro_rules! hlist {
    () => { HNil };
    ($a:expr) => { hlist![$a,] };
    ($a:expr, $($tok:tt)*) => {
        HCons {
            head: $a,
            tail: hlist![$($tok)*],
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! HList {
    () => { $crate::HNil };
    ($a:ty) => { $crate::HCons<$a, $crate::HNil> };
    ($a:ty, $($tok:tt)*) => {
        $crate::HCons<$a, $crate::HList![$($tok)*]>
    };
}

pub trait IntoHList {
    type Output: HList;
    fn into_hlist(self) -> Self::Output;
}

macro_rules! impl_tuple_into_hlist {
    ($($idx:tt $name:ident),*) => {
        impl<$($name),*> IntoHList for ($($name,)*) {
            type Output = HList![$($name),*];

            fn into_hlist(self) -> Self::Output {
                hlist![$(self.$idx),*]
            }
        }
    };
}

impl_tuple_into_hlist!(0 R0);
impl_tuple_into_hlist!(0 R0, 1 R1);
impl_tuple_into_hlist!(0 R0, 1 R1, 2 R2);
impl_tuple_into_hlist!(0 R0, 1 R1, 2 R2, 3 R3);
impl_tuple_into_hlist!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4);
impl_tuple_into_hlist!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5);
impl_tuple_into_hlist!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6);
impl_tuple_into_hlist!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7);
impl_tuple_into_hlist!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7, 8 R8);
impl_tuple_into_hlist!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7, 8 R8, 9 R9);
impl_tuple_into_hlist!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7, 8 R8, 9 R9, 10 R10);
impl_tuple_into_hlist!(0 R0, 1 R1, 2 R2, 3 R3, 4 R4, 5 R5, 6 R6, 7 R7, 8 R8, 9 R9, 10 R10, 11 R11);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_hlist() {
        let tuple = (1, "two", 3.0);
        let hlist = tuple.into_hlist();
        assert_eq!(hlist, hlist![1, "two", 3.0]);
    }
}
