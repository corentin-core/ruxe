//! `trybuild` runner for compile-fail tests in `tests/ui/`.
//!
//! Each `tests/ui/*.rs` file is a standalone Rust program that ruxe expects
//! to **fail** to compile. The expected error message is captured in a
//! companion `.stderr` file. `trybuild` invokes `rustc` on each file and
//! compares the actual error output against the `.stderr`.
//!
//! ## Updating `.stderr` files
//!
//! When error messages change (e.g. after refactoring internal traits),
//! re-bless the expected output:
//!
//! ```sh
//! TRYBUILD=overwrite cargo test --test compile_fail
//! ```
//!
//! Then re-run normally to confirm the tests pass against the new output.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
