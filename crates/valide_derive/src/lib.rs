//! Derive macros that generate the validation code of the `valide` crate.
//!
//! Use the macros through the `valide` crate, which re-exports them next to the traits they implement.

use proc_macro::TokenStream;

// The compilation suite and its fixtures need these crates, which this target never names
#[cfg(test)]
use {serde as _, serde_json as _, thiserror as _, trybuild as _, valide as _};

use crate::{
    expand::{expand_patch, expand_validate},
    input::parse,
};

mod expand;
mod input;
mod intermediate_representation;
mod naming;
mod range_text;

/// Generate the `Validate` implementation of the annotated type.
/// The `validate`, `final_validation` and `draft_attr` helper attributes drive the generation.
///
/// The macro generates the field enum, the validation error enum, the draft mirror, one validator per field,
/// the aggregate validator, the `TryFrom` of the draft, the `new` constructor and the field getters.
#[proc_macro_derive(Validate, attributes(validate, final_validation, draft_attr))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    let derive_input = syn::parse_macro_input!(input as syn::DeriveInput);

    match parse(&derive_input) {
        Ok(type_ir) => expand_validate(&type_ir).into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// Generate the `Patch` implementation of the annotated type.
/// The `validate`, `final_validation` and `draft_attr` helper attributes drive the generation.
///
/// The macro generates the conversion back to a draft and one validated setter per field.
/// The type must also derive `Validate`.
#[proc_macro_derive(Patch, attributes(validate, final_validation, draft_attr))]
pub fn derive_patch(input: TokenStream) -> TokenStream {
    let derive_input = syn::parse_macro_input!(input as syn::DeriveInput);

    match parse(&derive_input) {
        Ok(type_ir) => expand_patch(&type_ir).into(),
        Err(error) => error.into_compile_error().into(),
    }
}
