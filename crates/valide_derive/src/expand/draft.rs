//! Generation of the draft mirror of a validated type.
//!
//! The draft mirrors every field with its documentation and with the attributes that the field forwards,
//! the serde attributes and the payloads of the field level `draft_attr`.
//! Every field is public, so a caller can build a draft field by field and test it.
//! The draft of an enum mirrors every variant the same way, and every variant of an enum is already public.
//! A struct variant keeps its braces and its named fields, so an empty one stays distinct from a unit variant.
//! A nested field and a nested variant payload take the draft type of their own type, which the associated type projection names.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::{
    expand::{ExpansionContext, article, doc, validate_trait},
    intermediate_representation::{
        FieldIntermediateRepresentation, FieldRule, Shape, VariantFieldIntermediateRepresentation,
        VariantIntermediateRepresentation, VariantKind, VariantRule,
    },
};

/// Generate the draft of the validated type of `context`.
pub(crate) fn expand(context: &ExpansionContext<'_>) -> TokenStream {
    let intermediate_representation = context.intermediate_representation();
    let vis = &intermediate_representation.vis;
    let type_ident = &intermediate_representation.ident;
    let draft_ident = &intermediate_representation.draft_ident;
    // The declaration carries the generics of the validated type with their bounds and their defaults,
    // which the split spelling of an implementation drops
    let generics = &intermediate_representation.generics;
    let where_clause = context.where_clause();
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
/// A unit variant stays a unit variant, a payload variant holds the draft type of its payload
/// and a struct variant holds one field per field, each with the draft type of its own.
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
        VariantKind::Struct { fields } => {
            let draft_fields = fields.iter().map(|field| {
                let docs = variant_field_docs(field);
                let field_passthrough = field
                    .passthrough
                    .iter()
                    .map(|payload| quote! { #[#payload] });
                let field_ident = &field.ident;
                let field_type = payload_type(&field.ty, &field.rule);

                quote! {
                    #docs
                    #(#field_passthrough)*
                    #field_ident: #field_type,
                }
            });

            quote! {
                #attributes
                #ident { #(#draft_fields)* },
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

/// Return the documentation attributes that the draft gives to the given struct variant `field`.
/// A field of the validated type that carries no documentation gets a fallback,
/// because a named field of the draft is as public as the draft and must stay documented.
fn variant_field_docs(field: &VariantFieldIntermediateRepresentation) -> TokenStream {
    if field.docs.is_empty() {
        return doc(&format!("The `{}` field.", field.logical_name));
    }
    let docs = &field.docs;

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

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::{DeriveInput, parse_str};

    use crate::{
        expand::{ExpansionContext, draft::expand},
        input::parse,
    };

    /// Return the text of the given `tokens` with every space removed.
    fn token_text(tokens: &TokenStream) -> String {
        tokens.to_string().split_whitespace().collect()
    }

    /// Return the token text of the draft that the derive generates for `source`.
    /// The source must parse as a derive input and the derive must accept it.
    fn generated_draft(source: &str) -> String {
        let derive_input: DeriveInput =
            parse_str(source).expect("the tested derive input must parse");
        let intermediate_representation =
            parse(&derive_input).expect("the tested derive input must be accepted");
        let context = ExpansionContext::new(&intermediate_representation);

        token_text(&expand(&context))
    }

    #[test]
    fn the_draft_of_an_empty_struct_variant_keeps_its_braces() {
        let expected = quote! {
            #[doc = "Unvalidated draft of a [`Pointing`]."]
            enum PointingDraft {
                #[doc = "The `Nadir` variant."]
                Nadir,
                #[doc = "The `SunPointing` variant."]
                SunPointing {},
            }
        };

        assert_eq!(
            generated_draft("enum Pointing { Nadir, SunPointing {} }"),
            token_text(&expected),
            "The draft must keep the braces of an empty struct variant, so it stays distinct from a unit variant"
        );
    }

    #[test]
    fn the_draft_of_a_struct_variant_mirrors_its_fields() {
        let expected = quote! {
            #[doc = "Unvalidated draft of a [`Pointing`]."]
            enum PointingDraft {
                #[doc = "The `Custom` variant."]
                Custom {
                    #[doc = " Body to point at."]
                    #[serde(rename = "target")]
                    body: <CelestialBody as ::valide::Validate>::Draft,
                    #[doc = "The `comment` field."]
                    #[serde(default)]
                    comment: String,
                },
            }
        };

        assert_eq!(
            generated_draft(
                "enum Pointing { Custom { \
                 #[doc = \" Body to point at.\"] #[serde(rename = \"target\")] #[validate(nested)] body: CelestialBody, \
                 #[draft_attr(serde(default))] #[validate(skip)] comment: String } }"
            ),
            token_text(&expected),
            "The draft field must carry the documentation, the serde attribute and the draft_attr payload of its field, and a field without documentation must get the fallback"
        );
    }
}
