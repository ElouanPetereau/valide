//! Generation of the validation functions of a draft.
//!
//! For structs, the generator emits one field validator per validated field, so a caller can test the fields one by one.
//! It emits the aggregate that runs them in declaration order, then runs every final validation.
//! For enums, the aggregate matches the draft variant and delegates to the payload of the variant,
//! It emits no standalone validator, because delegation is the only rule that a variant carries.
//! The aggregate stops at the first error found.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    expand::{ExpansionContext, doc, validate_trait},
    intermediate_representation::{
        FieldIntermediateRepresentation, FieldRule, Shape, TypeIntermediateRepresentation,
        VariantKind,
    },
};

/// Generate the validation functions of the draft of the validated type of `context`.
pub(crate) fn expand(context: &ExpansionContext<'_>) -> TokenStream {
    let intermediate_representation = context.intermediate_representation();
    let vis = &intermediate_representation.vis;
    let draft_ident = &intermediate_representation.draft_ident;
    let impl_generics = context.impl_generics();
    let ty_generics = context.ty_generics();
    let where_clause = context.where_clause();
    let error_enum_type = context.error_type();
    let aggregate_doc = match intermediate_representation.shape {
        Shape::Named | Shape::Newtype => {
            doc("Validate every field of the draft with a fail fast policy.")
        }
        Shape::Enum => doc("Validate the payload of the draft variant."),
    };
    let return_doc = doc("Return the first error found.");

    let checks = match intermediate_representation.shape {
        Shape::Named | Shape::Newtype => field_calls(intermediate_representation),
        Shape::Enum => variant_checks(context),
    };
    let final_calls = context.final_validation_calls(&quote! { self });
    let validators = intermediate_representation
        .fields
        .iter()
        .filter_map(|field| field_validator(context, field));

    quote! {
        impl #impl_generics #draft_ident #ty_generics #where_clause {
            #aggregate_doc
            #return_doc
            #vis fn validate(&self) -> ::core::result::Result<(), #error_enum_type> {
                #checks
                #final_calls

                ::core::result::Result::Ok(())
            }

            #(#validators)*
        }
    }
}

/// Generate the calls of the field validators of `intermediate_representation`, in declaration order.
/// A skip field carries no validator, so it gets no call.
fn field_calls(intermediate_representation: &TypeIntermediateRepresentation) -> TokenStream {
    let calls = intermediate_representation
        .fields
        .iter()
        .filter(|field| !matches!(field.rule, FieldRule::Skip))
        .map(|field| {
            let validator = field.validator_ident();

            quote! { self.#validator()?; }
        });

    quote! { #(#calls)* }
}

/// Generate the match that validates the payload of the draft variant of the validated type of `context`.
/// A nested variant delegates to the type of its payload and wraps the error that the type returns.
/// Every variant that carries no rule shares a single arm, so no two arms of the match hold the same body.
/// Return an empty stream for an enum without a variant, which carries no payload to reach.
fn variant_checks(context: &ExpansionContext<'_>) -> TokenStream {
    let intermediate_representation = context.intermediate_representation();
    if intermediate_representation.variants.is_empty() {
        return TokenStream::new();
    }

    let validate = validate_trait();
    let error_constructor = context.error_constructor_turbofish();
    let mut nested_arms: Vec<TokenStream> = Vec::new();
    let mut plain_patterns: Vec<TokenStream> = Vec::new();
    for variant in &intermediate_representation.variants {
        let variant_ident = &variant.ident;
        if let Some((payload_type, wrapper_variant)) = variant.nested_payload() {
            nested_arms.push(quote! {
                Self::#variant_ident(payload) => {
                    ::core::result::Result::map_err(
                        <#payload_type as #validate>::validate(payload),
                        #error_constructor::#wrapper_variant,
                    )?;
                }
            });

            continue;
        }
        plain_patterns.push(match &variant.kind {
            VariantKind::Unit => quote! { Self::#variant_ident },
            VariantKind::Payload { .. } => quote! { Self::#variant_ident(_) },
        });
    }
    let plain_arm = (!plain_patterns.is_empty()).then(|| quote! { #(#plain_patterns)|* => {} });

    quote! {
        match self {
            #(#nested_arms)*
            #plain_arm
        }
    }
}

/// Generate the field validator of the given `field` of the validated type of `context`.
/// Return nothing for a skip field, which takes part in no validation at all.
fn field_validator(
    context: &ExpansionContext<'_>,
    field: &FieldIntermediateRepresentation,
) -> Option<TokenStream> {
    // A skip field carries no validator at all, so nothing below is generated for it
    if matches!(field.rule, FieldRule::Skip) {
        return None;
    }

    let intermediate_representation = context.intermediate_representation();
    let vis = &intermediate_representation.vis;
    // A variant struct literal reads its own generic arguments from the return type of the validator,
    // so it names the error enum without them
    let error_ident = &intermediate_representation.error_ident;
    let error_enum_type = context.error_type();
    let field_enum_ident = &intermediate_representation.field_enum_ident;
    let member = &field.member;
    let validator = field.validator_ident();
    let validator_doc = doc(&format!("Validate the `{}` field.", field.logical_name));

    let body = match &field.rule {
        // The early return above already handled this rule
        FieldRule::Skip => return None,
        FieldRule::Range { check_tokens, text } => {
            let variant = field.enum_variant();

            quote! {
                if !::core::ops::RangeBounds::contains(&(#check_tokens), &self.#member) {
                    return ::core::result::Result::Err(#error_ident::OutOfRange {
                        field: #field_enum_ident::#variant,
                        range: #text,
                    });
                }
            }
        }
        FieldRule::Finite => {
            let variant = field.enum_variant();

            quote! {
                if !self.#member.is_finite() {
                    return ::core::result::Result::Err(#error_ident::NotFinite {
                        field: #field_enum_ident::#variant,
                    });
                }
            }
        }
        FieldRule::Nested { wrapper_variant } => {
            let ty = &field.ty;
            let validate = validate_trait();
            let error_constructor = context.error_constructor_turbofish();

            quote! {
                ::core::result::Result::map_err(
                    <#ty as #validate>::validate(&self.#member),
                    #error_constructor::#wrapper_variant,
                )?;
            }
        }
        // The function belongs to the validated type and reads the field of the draft by reference,
        // which the call shape of a final validation already follows
        FieldRule::Custom {
            fn_ident,
            wrapper_variant,
            ..
        } => {
            let type_ident = &intermediate_representation.ident;
            let turbofish = context.turbofish();
            let error_constructor = context.error_constructor_turbofish();

            quote! {
                ::core::result::Result::map_err(
                    #type_ident #turbofish::#fn_ident(&self.#member),
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
