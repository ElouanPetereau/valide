//! Generation of the field enum of a validated type and of its [`Display`](core::fmt::Display) implementation.
//!
//! The enum has one variant per range or finite validated field.
//! The shared error variants carry one of its variants to name the field that failed.
//! A type without such a field gets no enum at all, which keeps every generated state reachable.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    expand::{article, doc, variant_of},
    intermediate_representation::{
        FieldIntermediateRepresentation, TypeIntermediateRepresentation,
    },
};

/// Generate the field enum of `intermediate_representation` and its [`Display`](core::fmt::Display) implementation.
/// Return an empty stream when no field of `intermediate_representation` can name itself inside an error.
pub(crate) fn expand(intermediate_representation: &TypeIntermediateRepresentation) -> TokenStream {
    let named_fields: Vec<&FieldIntermediateRepresentation> = intermediate_representation
        .fields
        .iter()
        .filter(|field| field.variant.is_some())
        .collect();
    if named_fields.is_empty() {
        return TokenStream::new();
    }

    let vis = &intermediate_representation.vis;
    let type_ident = &intermediate_representation.ident;
    let field_enum_ident = &intermediate_representation.field_enum_ident;
    let enum_doc = doc(&format!(
        "Validated fields of {} [`{type_ident}`].",
        article(&type_ident.to_string())
    ));

    let variants = named_fields.iter().map(|field| {
        let variant = variant_of(field);
        let variant_doc = doc(&format!("The `{}` field.", field.logical_name));

        quote! {
            #variant_doc
            #variant,
        }
    });
    let arms = named_fields.iter().map(|field| {
        let variant = variant_of(field);
        let logical_name = &field.logical_name;

        quote! { Self::#variant => #logical_name, }
    });

    quote! {
        #enum_doc
        #[derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::cmp::PartialEq,
            ::core::cmp::Eq,
            ::core::fmt::Debug
        )]
        #vis enum #field_enum_ident {
            #(#variants)*
        }

        impl ::core::fmt::Display for #field_enum_ident {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(
                    f,
                    match self {
                        #(#arms)*
                    },
                )
            }
        }
    }
}
