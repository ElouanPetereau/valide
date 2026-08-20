//! Generation of the draft mirror of a validated type.
//!
//! The draft mirrors every field with its documentation and with the attributes that the field forwards,
//! the serde attributes and the payloads of the field level `draft_attr`.
//! Every field is public, so a caller can build a draft field by field and test it.
//! The draft of an enum mirrors every variant the same way, and every variant of an enum is already public.
//! A nested field and a nested variant payload take the draft type of their own type, which the associated type projection names.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::{
    expand::{article, doc, validate_trait},
    intermediate_representation::{
        FieldIntermediateRepresentation, FieldRule, Shape, TypeIntermediateRepresentation,
        VariantIntermediateRepresentation, VariantKind, VariantRule,
    },
};

/// Generate the draft of `intermediate_representation`.
pub(crate) fn expand(intermediate_representation: &TypeIntermediateRepresentation) -> TokenStream {
    let vis = &intermediate_representation.vis;
    let type_ident = &intermediate_representation.ident;
    let draft_ident = &intermediate_representation.draft_ident;
    // The declaration carries the generics of the validated type with their bounds and their defaults,
    // which the split spelling of an implementation drops
    let generics = &intermediate_representation.generics;
    let (_, _, where_clause) = intermediate_representation.generics.split_for_impl();
    let draft_doc = doc(&format!(
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
        #draft_doc
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
                #vis struct #draft_ident #generics #where_clause {
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

            // The where clause of a tuple struct comes after the fields and before the semicolon
            quote! {
                #attributes
                #vis struct #draft_ident #generics (#(#fields),*) #where_clause;
            }
        }
        Shape::Enum => {
            let variants = intermediate_representation
                .variants
                .iter()
                .map(draft_variant);

            quote! {
                #attributes
                #vis enum #draft_ident #generics #where_clause {
                    #(#variants)*
                }
            }
        }
    }
}

/// Return the declaration that the draft gives to the given `variant`.
/// A unit variant stays a unit variant and a payload variant holds the draft type of its payload.
fn draft_variant(variant: &VariantIntermediateRepresentation) -> TokenStream {
    let ident = &variant.ident;
    let docs = variant_docs(variant);
    let passthrough = variant
        .passthrough
        .iter()
        .map(|payload| quote! { #[#payload] });
    let attributes = quote! {
        #docs
        #(#passthrough)*
    };

    match &variant.kind {
        VariantKind::Unit => quote! {
            #attributes
            #ident,
        },
        VariantKind::Payload {
            ty,
            rule,
            docs,
            passthrough,
        } => {
            let payload_passthrough = passthrough.iter().map(|payload| quote! { #[#payload] });
            let payload_type = payload_type(ty, rule);

            quote! {
                #attributes
                #ident(#(#docs)* #(#payload_passthrough)* #payload_type),
            }
        }
    }
}

/// Return the documentation attributes that the draft gives to the given `variant`.
/// A variant of the validated type that carries no documentation gets a fallback,
/// because every variant of the draft is public and must stay documented.
fn variant_docs(variant: &VariantIntermediateRepresentation) -> TokenStream {
    if variant.docs.is_empty() {
        return doc(&format!("The `{}` variant.", variant.ident));
    }
    let docs = &variant.docs;

    quote! { #(#docs)* }
}

/// Return the type that the draft gives to a variant payload of type `ty` under the rule `rule`.
/// A nested payload holds the draft of its own type, every other payload holds its declared type.
fn payload_type(ty: &Type, rule: &VariantRule) -> TokenStream {
    match rule {
        VariantRule::Nested { .. } => {
            let validate = validate_trait();

            quote! { <#ty as #validate>::Draft }
        }
        VariantRule::Skip => quote! { #ty },
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
    if matches!(field.rule, FieldRule::Nested { .. }) {
        let validate = validate_trait();

        return quote! { <#ty as #validate>::Draft };
    }

    quote! { #ty }
}
