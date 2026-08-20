//! Generation of the construction surface of a validated type.
//!
//! The generator emits the [`TryFrom`](core::convert::TryFrom) of its draft, the `new` constructor, the field getters and the [`Validate`](crate::Validate) implementation.
//! An enum gets no getter, because a caller matches on the public variants of the enum.
//! A value can only exist once a draft passed the whole validation.
//! The unchecked constructor that builds the type from a draft while skipping the validation
//! moves the fields of the draft and recurses into every nested field.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    expand::{doc, error_type, field_getter_ident, is_returned_by_value, validate_trait},
    intermediate_representation::{
        FieldIntermediateRepresentation, FieldRule, Shape, TypeIntermediateRepresentation,
        VariantKind, VariantRule,
    },
};

/// Generate the construction surface of `intermediate_representation`.
pub(crate) fn expand(intermediate_representation: &TypeIntermediateRepresentation) -> TokenStream {
    let vis = &intermediate_representation.vis;
    let type_ident = &intermediate_representation.ident;
    let draft_ident = &intermediate_representation.draft_ident;
    let (impl_generics, ty_generics, where_clause) =
        intermediate_representation.generics.split_for_impl();
    let turbofish = ty_generics.as_turbofish();
    let error_enum_type = error_type(intermediate_representation);
    let validate = validate_trait();

    let new_doc = doc(&format!(
        "Create a new [`{type_ident}`] from the given `draft`."
    ));
    let new_return_doc = doc("Return the first error found in the `draft`.");
    let getters = intermediate_representation
        .fields
        .iter()
        .map(|field| field_getter(intermediate_representation, field));
    let unchecked_body = unchecked_body(intermediate_representation);

    quote! {
        impl #impl_generics ::core::convert::TryFrom<#draft_ident #ty_generics> for #type_ident #ty_generics
        #where_clause
        {
            type Error = #error_enum_type;

            fn try_from(value: #draft_ident #ty_generics) -> ::core::result::Result<Self, Self::Error> {
                <Self as #validate>::from_draft(value)
            }
        }

        impl #impl_generics #type_ident #ty_generics #where_clause {
            #new_doc
            #new_return_doc
            #vis fn new(draft: #draft_ident #ty_generics) -> ::core::result::Result<Self, #error_enum_type> {
                <Self as ::core::convert::TryFrom<#draft_ident #ty_generics>>::try_from(draft)
            }

            #(#getters)*
        }

        impl #impl_generics #validate for #type_ident #ty_generics #where_clause {
            type Draft = #draft_ident #ty_generics;
            type Error = #error_enum_type;

            fn validate(draft: &Self::Draft) -> ::core::result::Result<(), Self::Error> {
                #draft_ident #turbofish::validate(draft)
            }

            fn from_draft_unchecked(draft: Self::Draft) -> Self {
                #unchecked_body
            }
        }
    }
}

/// Generate the getter of the given `field` of `intermediate_representation`.
/// A primitive field returns its value, every other field returns a reference to it.
fn field_getter(
    intermediate_representation: &TypeIntermediateRepresentation,
    field: &FieldIntermediateRepresentation,
) -> TokenStream {
    let vis = &intermediate_representation.vis;
    let ty = &field.ty;
    let member = &field.member;
    let getter = field_getter_ident(field);
    let getter_doc = doc(&format!("Retrieve the `{}` field.", field.logical_name));

    if is_returned_by_value(ty) {
        return quote! {
            #getter_doc
            #vis fn #getter(&self) -> #ty {
                self.#member
            }
        };
    }

    quote! {
        #getter_doc
        #vis fn #getter(&self) -> &#ty {
            &self.#member
        }
    }
}

/// Generate the body of the unchecked constructor of `intermediate_representation`.
fn unchecked_body(intermediate_representation: &TypeIntermediateRepresentation) -> TokenStream {
    match intermediate_representation.shape {
        Shape::Named => {
            let assignments = intermediate_representation.fields.iter().map(|field| {
                let member = &field.member;
                let value = field_value(field);

                quote! { #member: #value, }
            });

            quote! { Self { #(#assignments)* } }
        }
        Shape::Newtype => {
            let values = intermediate_representation.fields.iter().map(field_value);

            quote! { Self(#(#values),*) }
        }
        Shape::Enum => {
            let draft_ident = &intermediate_representation.draft_ident;
            let validate = validate_trait();
            // The scrutinee carries the generic arguments of the draft, so the patterns name the draft alone
            let arms = intermediate_representation.variants.iter().map(|variant| {
                let variant_ident = &variant.ident;
                let VariantKind::Payload { ty, rule, .. } = &variant.kind else {
                    return quote! { #draft_ident::#variant_ident => Self::#variant_ident, };
                };

                match rule {
                    VariantRule::Nested { .. } => quote! {
                        #draft_ident::#variant_ident(payload) => Self::#variant_ident(
                            <#ty as #validate>::from_draft_unchecked(payload),
                        ),
                    },
                    VariantRule::Skip => quote! {
                        #draft_ident::#variant_ident(payload) => Self::#variant_ident(payload),
                    },
                }
            });

            quote! {
                match draft {
                    #(#arms)*
                }
            }
        }
    }
}

/// Generate the value that the unchecked constructor moves into the given `field`.
/// A nested field builds its own type from its own draft.
fn field_value(field: &FieldIntermediateRepresentation) -> TokenStream {
    let member = &field.member;
    if matches!(field.rule, FieldRule::Nested { .. }) {
        let ty = &field.ty;
        let validate = validate_trait();

        return quote! { <#ty as #validate>::from_draft_unchecked(draft.#member) };
    }

    quote! { draft.#member }
}
