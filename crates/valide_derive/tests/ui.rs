//! Compilation suite of the `derive_validate` and `derive_patch` macros.

// The fixtures of the suite are crates of their own, so this target uses none of the dependencies that they need
use proc_macro2 as _;
use quote as _;
use serde as _;
use serde_json as _;
use syn as _;
use thiserror as _;
use valide as _;
use valide_derive as _;

#[cfg(test)]
mod ui {
    /// Check that every input of the fail suite is rejected with the expected diagnostics and
    /// that every input of the pass suite compiles and runs from a crate that is not `valide`.
    #[test]
    fn compilation_suite() {
        let test_cases = trybuild::TestCases::new();
        test_cases.compile_fail("tests/ui/fail/*.rs");
        test_cases.pass("tests/ui/pass/*.rs");
    }
}
