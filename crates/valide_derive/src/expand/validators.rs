//! Generation of the validation functions of a draft.
//!
//! The generator emits one field validator per validated field, so a caller can test the fields one by one.
//! It also emits the aggregate that runs them in declaration order, then runs every final validation.
//! The aggregate stops at the first error found.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    expand::{
        doc, error_constructor_turbofish, error_type, final_validation_calls, validate_trait,
        validator_ident, variant_of,
    },
    intermediate_representation::{
        FieldIntermediateRepresentation, Rule, TypeIntermediateRepresentation,
    },
};

/// Generate the validation functions of the draft of `intermediate_representation`.
pub(crate) fn expand(intermediate_representation: &TypeIntermediateRepresentation) -> TokenStream {
    let vis = &intermediate_representation.vis;
    let draft_ident = &intermediate_representation.draft_ident;
    let (impl_generics, ty_generics, where_clause) =
        intermediate_representation.generics.split_for_impl();
    let error_enum_type = error_type(intermediate_representation);
    let aggregate_doc = doc("Validate every field of the draft with a fail fast policy.");
    let return_doc = doc("Return the first error found.");

    let field_calls = intermediate_representation
        .fields
        .iter()
        .filter(|field| !matches!(field.rule, Rule::Skip))
        .map(|field| {
            let validator = validator_ident(field);

            quote! { self.#validator()?; }
        });
    let final_calls = final_validation_calls(intermediate_representation, &quote! { self });
    let validators = intermediate_representation
        .fields
        .iter()
        .filter_map(|field| validator(intermediate_representation, field));

    quote! {
        impl #impl_generics #draft_ident #ty_generics #where_clause {
            #aggregate_doc
            #return_doc
            #vis fn validate(&self) -> ::core::result::Result<(), #error_enum_type> {
                #(#field_calls)*
                #final_calls

                ::core::result::Result::Ok(())
            }

            #(#validators)*
        }
    }
}

/// Generate the field validator of the given `field` of `intermediate_representation`.
/// Return nothing for a skip field, which takes part in no validation at all.
fn validator(
    intermediate_representation: &TypeIntermediateRepresentation,
    field: &FieldIntermediateRepresentation,
) -> Option<TokenStream> {
    // A skip field carries no validator at all, so nothing below is generated for it
    if matches!(field.rule, Rule::Skip) {
        return None;
    }

    let vis = &intermediate_representation.vis;
    // A variant struct literal reads its own generic arguments from the return type of the validator,
    // so it names the error enum without them
    let error_ident = &intermediate_representation.error_ident;
    let error_enum_type = error_type(intermediate_representation);
    let field_enum_ident = &intermediate_representation.field_enum_ident;
    let member = &field.member;
    let validator = validator_ident(field);
    let validator_doc = doc(&format!("Validate the `{}` field.", field.logical_name));

    let body = match &field.rule {
        // The early return above already handled this rule
        Rule::Skip => return None,
        Rule::Range { check_tokens, text } => {
            let variant = variant_of(field);

            quote! {
                if !::core::ops::RangeBounds::contains(&(#check_tokens), &self.#member) {
                    return ::core::result::Result::Err(#error_ident::OutOfRange {
                        field: #field_enum_ident::#variant,
                        range: #text,
                    });
                }
            }
        }
        Rule::Finite => {
            let variant = variant_of(field);

            quote! {
                if !self.#member.is_finite() {
                    return ::core::result::Result::Err(#error_ident::NotFinite {
                        field: #field_enum_ident::#variant,
                    });
                }
            }
        }
        Rule::Nested { wrapper_variant } => {
            let ty = &field.ty;
            let validate = validate_trait();
            let error_constructor = error_constructor_turbofish(intermediate_representation);

            quote! {
                ::core::result::Result::map_err(
                    <#ty as #validate>::validate(&self.#member),
                    #error_constructor::#wrapper_variant,
                )?;
            }
        }
    };

    Some(quote! {
        #validator_doc
        #vis fn #validator(&self) -> ::core::result::Result<(), #error_enum_type> {
            #body

            ::core::result::Result::Ok(())
        }
    })
}
