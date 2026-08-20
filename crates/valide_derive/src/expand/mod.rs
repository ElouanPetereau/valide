//! Code generation of every item that the two derives produce.
//!
//! The module holds the helpers that the expanders share.
//! Every emitted path is absolute, so the generated code never depends on the imports of the module that holds the validated type.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Type, spanned::Spanned as _};

use crate::intermediate_representation::{FieldRule, TypeIntermediateRepresentation};

pub(crate) mod construction;
pub(crate) mod draft;
pub(crate) mod error_enum;
pub(crate) mod field_enum;
pub(crate) mod patch;
pub(crate) mod validators;

/// Generate every item of the `Validate` derive of `intermediate_representation`.
/// The bound assertions come first, so their targeted diagnostic reaches the reader before the diagnostics of the generated items.
pub(crate) fn expand_validate(
    intermediate_representation: &TypeIntermediateRepresentation,
) -> TokenStream {
    let assertions = nested_assertions(intermediate_representation, &validate_trait());
    let error_assertions = final_validation_error_assertions(intermediate_representation);
    let field_enum = field_enum::expand(intermediate_representation);
    let error_enum = error_enum::expand(intermediate_representation);
    let draft = draft::expand(intermediate_representation);
    let validators = validators::expand(intermediate_representation);
    let construction = construction::expand(intermediate_representation);

    quote! {
        #assertions
        #error_assertions
        #field_enum
        #error_enum
        #draft
        #validators
        #construction
    }
}

/// Generate every item of the `Patch` derive of `intermediate_representation`.
/// The bound assertions come first, so their targeted diagnostic reaches the reader before the diagnostics of the generated items.
pub(crate) fn expand_patch(
    intermediate_representation: &TypeIntermediateRepresentation,
) -> TokenStream {
    let assertions = nested_assertions(intermediate_representation, &patch_trait());
    let clone_assertion = clone_assertion(intermediate_representation);
    let items = patch::expand(intermediate_representation);

    quote! {
        #assertions
        #clone_assertion
        #items
    }
}

/// Path of the `Validate` trait in the generated code.
pub(crate) fn validate_trait() -> TokenStream {
    quote! { ::valide::Validate }
}

/// Path of the `Patch` trait in the generated code.
pub(crate) fn patch_trait() -> TokenStream {
    quote! { ::valide::Patch }
}

/// Return the validation error type of `intermediate_representation` as every type position names it.
/// The type carries the generic arguments of the validated type once a parameter reaches an error payload.
pub(crate) fn error_type(
    intermediate_representation: &TypeIntermediateRepresentation,
) -> TokenStream {
    let error_ident = &intermediate_representation.error_ident;
    if !intermediate_representation.error_enum_is_generic {
        return quote! { #error_ident };
    }
    let (_, ty_generics, _) = intermediate_representation.generics.split_for_impl();

    quote! { #error_ident #ty_generics }
}

/// Return the path that names a wrapper variant of the error enum of `intermediate_representation` as a function value.
/// Such a position carries no return type to infer the generic arguments from, so the path spells them out.
pub(crate) fn error_constructor_turbofish(
    intermediate_representation: &TypeIntermediateRepresentation,
) -> TokenStream {
    let error_ident = &intermediate_representation.error_ident;
    if !intermediate_representation.error_enum_is_generic {
        return quote! { #error_ident };
    }
    let (_, ty_generics, _) = intermediate_representation.generics.split_for_impl();
    let turbofish = ty_generics.as_turbofish();

    quote! { #error_ident #turbofish }
}

/// Build the documentation attribute that carries the given `text`.
pub(crate) fn doc(text: &str) -> TokenStream {
    quote! { #[doc = #text] }
}

/// Return the indefinite article of the given `name`.
pub(crate) fn article(name: &str) -> &'static str {
    match name.chars().next() {
        Some('A' | 'E' | 'I' | 'O' | 'U' | 'a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

/// Generate the calls that run every final validation of `intermediate_representation` on the draft named `draft_name`.
/// Each call wraps its error inside the wrapper variant of the final validation.
pub(crate) fn final_validation_calls(
    intermediate_representation: &TypeIntermediateRepresentation,
    draft_name: &TokenStream,
) -> TokenStream {
    let type_ident = &intermediate_representation.ident;
    let (_, ty_generics, _) = intermediate_representation.generics.split_for_impl();
    let turbofish = ty_generics.as_turbofish();
    let error_constructor = error_constructor_turbofish(intermediate_representation);
    let calls = intermediate_representation
        .final_validations
        .iter()
        .map(|final_validation| {
            let fn_ident = &final_validation.fn_ident;
            let wrapper_variant = &final_validation.wrapper_variant;

            quote! {
                ::core::result::Result::map_err(
                    #type_ident #turbofish::#fn_ident(#draft_name),
                    #error_constructor::#wrapper_variant,
                )?;
            }
        });

    quote! { #(#calls)* }
}

/// Generate the bound assertions of every nested type of `intermediate_representation` against `trait_path`.
/// The span of the nested type carries the diagnostic, so the compiler points at the field or at the variant payload.
/// The generic parameters of the validated type reach the assertion function, which binds the ones the nested type names.
fn nested_assertions(
    intermediate_representation: &TypeIntermediateRepresentation,
    trait_path: &TokenStream,
) -> TokenStream {
    let (impl_generics, _, where_clause) = intermediate_representation.generics.split_for_impl();
    let assertion_allow = assertion_allow();
    let assertions = nested_types(intermediate_representation).map(|ty| {
        quote_spanned! { ty.span()=>
            const _: () = {
                #assertion_allow
                fn assertion #impl_generics () #where_clause {
                    fn assert_nested_field<NestedType: #trait_path>() {}
                    assert_nested_field::<#ty>();
                }
            };
        }
    });

    quote! { #(#assertions)* }
}

/// Return the declared type of every nested field and of every nested variant payload of `intermediate_representation`.
/// Such a type carries a validation of its own, which the generated code delegates to.
fn nested_types(
    intermediate_representation: &TypeIntermediateRepresentation,
) -> impl Iterator<Item = &Type> {
    let field_types = intermediate_representation
        .fields
        .iter()
        .filter(|field| matches!(field.rule, FieldRule::Nested { .. }))
        .map(|field| &field.ty);
    let payload_types = intermediate_representation
        .variants
        .iter()
        .filter_map(|variant| {
            variant
                .nested_payload()
                .map(|(payload_type, _)| payload_type)
        });

    field_types.chain(payload_types)
}

/// Generate the assertion that every final validation error of `intermediate_representation` implements [`Error`](core::error::Error).
/// The generated error enum reports such an error as its source, which needs the trait.
/// The span of the error type carries the diagnostic, so the compiler points at the attribute.
/// The generic parameters of the validated type reach the assertion function, which binds the ones the error type names.
fn final_validation_error_assertions(
    intermediate_representation: &TypeIntermediateRepresentation,
) -> TokenStream {
    let (impl_generics, _, where_clause) = intermediate_representation.generics.split_for_impl();
    let assertion_allow = assertion_allow();
    let assertions = intermediate_representation
        .final_validations
        .iter()
        .map(|final_validation| {
            let error_ty = &final_validation.error_ty;
            // Only the call carries the span of the error type. The bound keeps the span of the macro,
            // so the absolute path of the trait stays out of the source of the caller
            let assertion_call = quote_spanned! { error_ty.span()=>
                assert_final_validation_error::<#error_ty>();
            };

            quote! {
                const _: () = {
                    #assertion_allow
                    fn assertion #impl_generics () #where_clause {
                        fn assert_final_validation_error<ErrorType: ::core::error::Error>() {}
                        #assertion_call
                    }
                };
            }
        });

    quote! { #(#assertions)* }
}

/// Generate the assertion that the type of `intermediate_representation` implements `Clone`, which every setter needs.
/// The assertion function carries the generics of the validated type, so the assertion holds under its own where clause.
fn clone_assertion(intermediate_representation: &TypeIntermediateRepresentation) -> TokenStream {
    let type_ident = &intermediate_representation.ident;
    let (impl_generics, ty_generics, where_clause) =
        intermediate_representation.generics.split_for_impl();
    let assertion_allow = assertion_allow();

    quote_spanned! { type_ident.span()=>
        const _: () = {
            #assertion_allow
            fn assertion #impl_generics () #where_clause {
                fn assert_patched_type<PatchedType: ::core::clone::Clone>() {}
                assert_patched_type::<#type_ident #ty_generics>();
            }
        };
    }
}

/// Build the attribute that lets an assertion function stay uncalled.
/// The three assertion shells share it, because a caller that never calls the function must see no warning.
/// An `expect` attribute would warn on its own every time the lint of the generated code stays quiet.
fn assertion_allow() -> TokenStream {
    quote! {
        #[allow(dead_code, reason = "the assertion function is type checked and never called")]
    }
}
