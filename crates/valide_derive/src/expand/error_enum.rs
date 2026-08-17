//! Generation of the validation error enum of a validated type.
//!
//! The generator emits the shared field shaped variants that carry the failing field.
//! It emits one wrapper variant per final validation and one per nested field.
//! A variant only exists when at least one field or one attribute can produce it.
//! A wrapper variant reports the error it holds as its source.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    expand::{doc, validate_trait},
    intermediate_representation::{Rule, TypeIntermediateRepresentation},
};

/// Generate the validation error enum of `intermediate_representation`, with its [`Display`](core::fmt::Display) and its [`Error`](core::error::Error).
pub(crate) fn expand(intermediate_representation: &TypeIntermediateRepresentation) -> TokenStream {
    let vis = &intermediate_representation.vis;
    let type_ident = &intermediate_representation.ident;
    let error_ident = &intermediate_representation.error_ident;
    let field_enum_ident = &intermediate_representation.field_enum_ident;
    let enum_doc = doc(&format!("Error type of the [`{type_ident}`] validation."));
    let field_doc = doc("The field that failed the validation.");

    let has_range = intermediate_representation
        .fields
        .iter()
        .any(|field| matches!(field.rule, Rule::Range { .. }));
    let has_finite = intermediate_representation
        .fields
        .iter()
        .any(|field| matches!(field.rule, Rule::Finite));

    let out_of_range = has_range.then(|| {
        let variant_doc = doc("The field value is outside its valid range.");
        let range_doc = doc("The valid range of the field.");

        quote! {
            #variant_doc
            OutOfRange {
                #field_doc
                field: #field_enum_ident,
                #range_doc
                range: &'static str,
            },
        }
    });
    let not_finite = has_finite.then(|| {
        let variant_doc = doc("The field value is not a finite number.");

        quote! {
            #variant_doc
            NotFinite {
                #field_doc
                field: #field_enum_ident,
            },
        }
    });

    let final_wrappers =
        intermediate_representation
            .final_validations
            .iter()
            .map(|final_validation| {
                let wrapper_variant = &final_validation.wrapper_variant;
                let error_ty = &final_validation.error_ty;
                let variant_doc = doc(&format!(
                    "The final validation `{}` failed.",
                    final_validation.fn_ident
                ));

                quote! {
                    #variant_doc
                    #wrapper_variant(#error_ty),
                }
            });
    let nested_wrappers = intermediate_representation
        .fields
        .iter()
        .filter_map(|field| {
            let Rule::Nested { wrapper_variant } = &field.rule else {
                return None;
            };
            let ty = &field.ty;
            let validate = validate_trait();
            let variant_doc = doc(&format!(
                "The validation of the `{}` field failed.",
                field.logical_name
            ));

            Some(quote! {
                #variant_doc
                #wrapper_variant(<#ty as #validate>::Error),
            })
        });

    let display_body = display_body(intermediate_representation, has_range, has_finite);
    let source_body = source_body(intermediate_representation, has_range, has_finite);

    quote! {
        #enum_doc
        // The enum carries no `Eq`, so an error type of a final validation can hold a float
        #[derive(::core::clone::Clone, ::core::cmp::PartialEq, ::core::fmt::Debug)]
        #vis enum #error_ident {
            #out_of_range
            #not_finite
            #(#final_wrappers)*
            #(#nested_wrappers)*
        }

        impl ::core::fmt::Display for #error_ident {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #display_body
            }
        }

        impl ::core::error::Error for #error_ident {
            fn source(&self) -> ::core::option::Option<&(dyn ::core::error::Error + 'static)> {
                #source_body
            }
        }
    }
}

/// Generate the body of the [`Display`](core::fmt::Display) implementation of the error enum of `intermediate_representation`.
fn display_body(
    intermediate_representation: &TypeIntermediateRepresentation,
    has_range: bool,
    has_finite: bool,
) -> TokenStream {
    let mut arms = Vec::new();
    if has_range {
        arms.push(quote! {
            Self::OutOfRange { field, range } => {
                ::core::write!(f, "The {field} must be within the range {range}")
            }
        });
    }
    if has_finite {
        arms.push(quote! {
            Self::NotFinite { field } => ::core::write!(f, "The {field} must be a finite number"),
        });
    }
    for wrapper_variant in wrapper_variants(intermediate_representation) {
        arms.push(quote! {
            Self::#wrapper_variant(error) => ::core::fmt::Display::fmt(error, f),
        });
    }
    // An enum without a variant holds no value, so the match needs no arm at all
    if arms.is_empty() {
        return quote! { match *self {} };
    }

    quote! {
        match self {
            #(#arms)*
        }
    }
}

/// Generate the body of the `source` function of the error enum of `intermediate_representation`.
/// Only a wrapper variant holds an error, so only a wrapper variant reports a source.
fn source_body(
    intermediate_representation: &TypeIntermediateRepresentation,
    has_range: bool,
    has_finite: bool,
) -> TokenStream {
    let mut arms = Vec::new();
    if has_range {
        arms.push(quote! { Self::OutOfRange { .. } => ::core::option::Option::None, });
    }
    if has_finite {
        arms.push(quote! { Self::NotFinite { .. } => ::core::option::Option::None, });
    }
    for wrapper_variant in wrapper_variants(intermediate_representation) {
        arms.push(quote! {
            Self::#wrapper_variant(error) => ::core::option::Option::Some(error),
        });
    }
    if arms.is_empty() {
        return quote! { match *self {} };
    }

    quote! {
        match self {
            #(#arms)*
        }
    }
}

/// Return the wrapper variants of `intermediate_representation`, the final validations first and the nested fields after.
/// The order matches the order of the variants of the generated enum.
fn wrapper_variants(
    intermediate_representation: &TypeIntermediateRepresentation,
) -> impl Iterator<Item = &proc_macro2::Ident> {
    let final_variants = intermediate_representation
        .final_validations
        .iter()
        .map(|final_validation| &final_validation.wrapper_variant);
    let nested_variants =
        intermediate_representation
            .fields
            .iter()
            .filter_map(|field| match &field.rule {
                Rule::Nested { wrapper_variant } => Some(wrapper_variant),
                Rule::Range { .. } | Rule::Finite | Rule::Skip => None,
            });

    final_variants.chain(nested_variants)
}
