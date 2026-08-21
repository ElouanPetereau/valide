//! Generation of the validation error enum of a validated type.
//!
//! The generator emits the shared field shaped variants that carry the failing field.
//! It emits one wrapper variant per nested variant of an enum, per final validation and per nested or custom field.
//! A variant only exists when at least one field, one variant or one attribute can produce it.
//! A wrapper variant reports the error it holds as its source.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    expand::{ExpansionContext, doc, validate_trait},
    intermediate_representation::{FieldRule, TypeIntermediateRepresentation},
};

/// Generate the validation error enum of the validated type of `context`, with its [`Display`](core::fmt::Display) and its [`Error`](core::error::Error).
pub(crate) fn expand(context: &ExpansionContext<'_>) -> TokenStream {
    let intermediate_representation = context.intermediate_representation();
    let vis = &intermediate_representation.vis;
    let type_ident = &intermediate_representation.ident;
    let error_ident = &intermediate_representation.error_ident;
    let field_enum_ident = &intermediate_representation.field_enum_ident;
    let enum_doc = doc(&format!("Error type of the [`{type_ident}`] validation."));
    let field_doc = doc("The field that failed the validation.");
    // Only a parameter that reaches a variant may reach the enum,
    // because an unused parameter of an enum is an error that the caller cannot fix
    let generic_declaration = generic_declaration(context);
    let (implementation_header, implementation_where_clause) = implementation_generics(context);
    let error_enum_type = context.error_type();

    let has_range = intermediate_representation.has_range();
    let has_finite = intermediate_representation.has_finite();

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

    let variant_wrappers = intermediate_representation
        .variants
        .iter()
        .filter_map(|variant| {
            let (payload_type, wrapper_variant) = variant.nested_payload()?;
            let validate = validate_trait();
            let variant_doc = doc(&format!(
                "The validation of the `{}` variant failed.",
                variant.ident
            ));

            Some(quote! {
                #variant_doc
                #wrapper_variant(<#payload_type as #validate>::Error),
            })
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
    // A nested field and a custom field share a single pass, so the two wrapper kinds keep the declaration order of the fields
    let field_wrappers = intermediate_representation
        .fields
        .iter()
        .filter_map(|field| match &field.rule {
            FieldRule::Nested { wrapper_variant } => {
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
            }
            FieldRule::Custom {
                fn_ident,
                error_ty,
                wrapper_variant,
            } => {
                let variant_doc = doc(&format!(
                    "The custom validation `{fn_ident}` of the `{}` field failed.",
                    field.logical_name
                ));

                Some(quote! {
                    #variant_doc
                    #wrapper_variant(#error_ty),
                })
            }
            FieldRule::Range { .. } | FieldRule::Finite | FieldRule::Skip => None,
        });

    let display_body = display_body(intermediate_representation, has_range, has_finite);
    let source_body = source_body(intermediate_representation, has_range, has_finite);

    quote! {
        #enum_doc
        // The enum carries no `Eq`, so an error type of a final validation can hold a float
        #[allow(
            clippy::derive_partial_eq_without_eq,
            reason = "a wrapped error type may hold a float, which the macro cannot know"
        )]
        #[derive(::core::clone::Clone, ::core::cmp::PartialEq, ::core::fmt::Debug)]
        #vis enum #error_ident #generic_declaration {
            #out_of_range
            #not_finite
            #(#variant_wrappers)*
            #(#final_wrappers)*
            #(#field_wrappers)*
        }

        impl #implementation_header ::core::fmt::Display for #error_enum_type
        #implementation_where_clause
        {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #display_body
            }
        }

        impl #implementation_header ::core::error::Error for #error_enum_type
        #implementation_where_clause
        {
            fn source(&self) -> ::core::option::Option<&(dyn ::core::error::Error + 'static)> {
                #source_body
            }
        }
    }
}

/// Return the generics that the declaration of the error enum of the validated type of `context` carries.
/// The declaration stays free of every generic parameter while no parameter reaches a variant.
fn generic_declaration(context: &ExpansionContext<'_>) -> Option<TokenStream> {
    let intermediate_representation = context.intermediate_representation();
    if !intermediate_representation.error_enum_is_generic {
        return None;
    }
    let generics = &intermediate_representation.generics;
    let where_clause = context.where_clause();

    Some(quote! { #generics #where_clause })
}

/// Return the header generics and the where clause that every implementation of the error enum of the validated type of `context` carries.
/// Both stay empty while the error enum is free of every generic parameter.
fn implementation_generics(context: &ExpansionContext<'_>) -> (TokenStream, TokenStream) {
    if !context.intermediate_representation().error_enum_is_generic {
        return (TokenStream::new(), TokenStream::new());
    }
    let impl_generics = context.impl_generics();
    let where_clause = context.where_clause();

    (quote! { #impl_generics }, quote! { #where_clause })
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
    for wrapper_variant in intermediate_representation.wrapper_variants() {
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
    for wrapper_variant in intermediate_representation.wrapper_variants() {
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
