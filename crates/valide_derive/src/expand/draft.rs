//! Generation of the draft mirror of a validated type.
//!
//! The draft mirrors every field with its documentation and with the attributes that the field forwards,
//! the serde attributes and the payloads of the field level `draft_attr`.
//! Every field is public, so a caller can build a draft field by field and test it.
//! A nested field takes the draft type of its own type, which the associated type projection names.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    expand::{article, doc, validate_trait},
    intermediate_representation::{
        FieldIntermediateRepresentation, Rule, Shape, TypeIntermediateRepresentation,
    },
};

/// Generate the draft of `intermediate_representation`.
pub(crate) fn expand(intermediate_representation: &TypeIntermediateRepresentation) -> TokenStream {
    let vis = &intermediate_representation.vis;
    let type_ident = &intermediate_representation.ident;
    let draft_ident = &intermediate_representation.draft_ident;
    let struct_doc = doc(&format!(
        "Unvalidated draft of {} [`{type_ident}`].",
        article(&type_ident.to_string())
    ));

    // The serde derives mirror the deserialization validation of the validated type.
    // A `cfg_attr` resolves before the derive runs, so the mirror follows the configuration on its own
    let serde_derives = intermediate_representation.emit_draft_serde.then(|| {
        quote! {
            #[derive(::serde::Serialize, ::serde::Deserialize)]
        }
    });
    let passthrough = intermediate_representation
        .draft_passthrough
        .iter()
        .map(|payload| quote! { #[#payload] });
    let attributes = quote! {
        #struct_doc
        #serde_derives
        #(#passthrough)*
    };

    match intermediate_representation.shape {
        Shape::Named => {
            let fields = intermediate_representation.fields.iter().map(|field| {
                let field_attributes = field_attributes(field);
                let member = &field.member;
                let ty = field_type(field);

                quote! {
                    #field_attributes
                    pub #member: #ty,
                }
            });

            quote! {
                #attributes
                #vis struct #draft_ident {
                    #(#fields)*
                }
            }
        }
        Shape::Newtype => {
            let fields = intermediate_representation.fields.iter().map(|field| {
                let field_attributes = field_attributes(field);
                let ty = field_type(field);

                quote! { #field_attributes pub #ty }
            });

            quote! {
                #attributes
                #vis struct #draft_ident(#(#fields),*);
            }
        }
    }
}

/// Return every attribute that the draft gives to the given `field`.
/// The documentation comes first, then the payloads that the field forwards to the draft.
fn field_attributes(field: &FieldIntermediateRepresentation) -> TokenStream {
    let docs = field_docs(field);
    let passthrough = field
        .passthrough
        .iter()
        .map(|payload| quote! { #[#payload] });

    quote! {
        #docs
        #(#passthrough)*
    }
}

/// Return the documentation attributes that the draft gives to the given `field`.
/// A field of the validated type that carries no documentation gets a fallback,
/// because every field of the draft is public and must stay documented.
fn field_docs(field: &FieldIntermediateRepresentation) -> TokenStream {
    if field.docs.is_empty() {
        return doc(&format!("The `{}` field.", field.logical_name));
    }
    let docs = &field.docs;

    quote! { #(#docs)* }
}

/// Return the type that the draft gives to the given `field`.
/// A nested field holds the draft of its own type, every other field holds its declared type.
fn field_type(field: &FieldIntermediateRepresentation) -> TokenStream {
    let ty = &field.ty;
    if matches!(field.rule, Rule::Nested { .. }) {
        let validate = validate_trait();

        return quote! { <#ty as #validate>::Draft };
    }

    quote! { #ty }
}
