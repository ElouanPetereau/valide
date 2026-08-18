//! Generation of the patch surface of a validated type.
//!
//! The generator emits the conversion back to a draft, the [`Patch`](crate::Patch) implementation and the field setters.
//! A setter validates a draft that carries the new value, then commits the value only on success,
//! so a rejected patch leaves the value untouched.
//! The setter of a skip field only stays infallible while the type declares no final validation.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    expand::{
        doc, error_type, final_validation_calls, new_value_ident, patch_trait, setter_ident,
        validator_ident,
    },
    intermediate_representation::{
        FieldIntermediateRepresentation, Rule, Shape, TypeIntermediateRepresentation,
    },
};

/// Generate the patch surface of `intermediate_representation`.
pub(crate) fn expand(intermediate_representation: &TypeIntermediateRepresentation) -> TokenStream {
    let type_ident = &intermediate_representation.ident;
    let draft_ident = &intermediate_representation.draft_ident;
    let (impl_generics, ty_generics, where_clause) =
        intermediate_representation.generics.split_for_impl();
    let patch = patch_trait();

    let conversion_body = conversion_body(intermediate_representation);
    let setters = intermediate_representation
        .fields
        .iter()
        .map(|field| setter(intermediate_representation, field));

    quote! {
        impl #impl_generics ::core::convert::From<#type_ident #ty_generics> for #draft_ident #ty_generics
        #where_clause
        {
            fn from(value: #type_ident #ty_generics) -> Self {
                #conversion_body
            }
        }

        impl #impl_generics #patch for #type_ident #ty_generics #where_clause {
            fn to_draft(&self) -> Self::Draft {
                <#draft_ident #ty_generics as ::core::convert::From<#type_ident #ty_generics>>::from(
                    ::core::clone::Clone::clone(self),
                )
            }
        }

        impl #impl_generics #type_ident #ty_generics #where_clause {
            #(#setters)*
        }
    }
}

/// Generate the body of the conversion of `intermediate_representation` back to its draft.
fn conversion_body(intermediate_representation: &TypeIntermediateRepresentation) -> TokenStream {
    match intermediate_representation.shape {
        Shape::Named => {
            let assignments = intermediate_representation.fields.iter().map(|field| {
                let member = &field.member;
                let value = draft_value(field);

                quote! { #member: #value, }
            });

            quote! { Self { #(#assignments)* } }
        }
        Shape::Newtype => {
            let values = intermediate_representation.fields.iter().map(draft_value);

            quote! { Self(#(#values),*) }
        }
    }
}

/// Generate the draft value of the given `field`, read from a value named `value`.
/// A nested field converts itself back to its own draft.
fn draft_value(field: &FieldIntermediateRepresentation) -> TokenStream {
    let member = &field.member;
    if matches!(field.rule, Rule::Nested { .. }) {
        let patch = patch_trait();

        return quote! { #patch::to_draft(&value.#member) };
    }

    quote! { value.#member }
}

/// Generate the setter of the given `field` of `intermediate_representation`.
fn setter(
    intermediate_representation: &TypeIntermediateRepresentation,
    field: &FieldIntermediateRepresentation,
) -> TokenStream {
    let vis = &intermediate_representation.vis;
    let type_ident = &intermediate_representation.ident;
    let draft_ident = &intermediate_representation.draft_ident;
    let (_, ty_generics, _) = intermediate_representation.generics.split_for_impl();
    let error_enum_type = error_type(intermediate_representation);
    let ty = &field.ty;
    let member = &field.member;
    let setter = setter_ident(field);
    let new_value = new_value_ident(field);
    let setter_doc = doc(&format!("Set the given `{new_value}`."));
    let is_skipped = matches!(field.rule, Rule::Skip);

    // A skip field carries no field validator,
    // so nothing can reject the new value while the type declares no final validation.
    // The setter then needs no draft and cannot fail
    if is_skipped && intermediate_representation.final_validations.is_empty() {
        return quote! {
            #setter_doc
            #vis fn #setter(&mut self, #new_value: #ty) {
                self.#member = #new_value;
            }
        };
    }

    let error_doc = doc(&format!("Return an error if `{new_value}` is not valid."));
    // A final validation can read a skip field,
    // so the setter of such a field builds the draft and runs the final validations.
    // It runs no field validator, because a skip field has none
    let validator_call = (!is_skipped).then(|| {
        let validator = validator_ident(field);

        quote! { let _: () = tmp_draft.#validator()?; }
    });
    let final_calls = final_validation_calls(intermediate_representation, &quote! { &tmp_draft });
    // A nested field lends its value to the draft, which holds the draft of the nested type,
    // so the commit still owns the new value.
    // Every other field moves its new value into the draft and the commit moves it back out once the validation passed
    let (draft_assignment, commit) = if matches!(field.rule, Rule::Nested { .. }) {
        let patch = patch_trait();

        (
            quote! { #patch::to_draft(&#new_value) },
            quote! { self.#member = #new_value; },
        )
    } else {
        (
            quote! { #new_value },
            quote! { self.#member = tmp_draft.#member; },
        )
    };

    quote! {
        #setter_doc
        #error_doc
        #vis fn #setter(
            &mut self,
            #new_value: #ty,
        ) -> ::core::result::Result<(), #error_enum_type> {
            let mut tmp_draft: #draft_ident #ty_generics = <#draft_ident #ty_generics as ::core::convert::From<#type_ident #ty_generics>>::from(
                ::core::clone::Clone::clone(self),
            );
            tmp_draft.#member = #draft_assignment;
            #validator_call
            #final_calls

            #commit

            ::core::result::Result::Ok(())
        }
    }
}
