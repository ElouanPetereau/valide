//! Code generation of every item that the two derives produce.
//!
//! The module holds the expansion context and the leaf helpers that the expanders share.
//! Every emitted path is absolute, so the generated code never depends on the imports of the module that holds the validated type.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{ImplGenerics, Turbofish, TypeGenerics, WhereClause, spanned::Spanned as _};

use crate::intermediate_representation::TypeIntermediateRepresentation;

pub(crate) mod construction;
pub(crate) mod draft;
pub(crate) mod error_enum;
pub(crate) mod field_enum;
pub(crate) mod patch;
pub(crate) mod validators;

/// Owner of the type level pieces that every expander needs.
/// The context holds the representation of the validated type and the split spellings of its generics.
/// Each derive builds one context and every expander reads it, so the generics discipline has a single owner.
pub(crate) struct ExpansionContext<'ir> {
    /// Representation of the validated type.
    intermediate_representation: &'ir TypeIntermediateRepresentation,
    /// Generics of the validated type as the header of an implementation spells them.
    impl_generics: ImplGenerics<'ir>,
    /// Generics of the validated type as a type position spells them.
    ty_generics: TypeGenerics<'ir>,
    /// Where clause of the validated type, absent while the type declares none.
    where_clause: Option<&'ir WhereClause>,
    /// Generic arguments of the validated type as an expression position spells them.
    turbofish: Turbofish<'ir>,
}

impl<'ir> ExpansionContext<'ir> {
    /// Build the context of the given `intermediate_representation`.
    pub(crate) fn new(intermediate_representation: &'ir TypeIntermediateRepresentation) -> Self {
        let (impl_generics, ty_generics, where_clause) =
            intermediate_representation.generics.split_for_impl();
        let turbofish = ty_generics.as_turbofish();

        Self {
            intermediate_representation,
            impl_generics,
            ty_generics,
            where_clause,
            turbofish,
        }
    }

    /// Return the representation of the validated type, which every field level and variant level generator walks.
    pub(crate) fn intermediate_representation(&self) -> &'ir TypeIntermediateRepresentation {
        self.intermediate_representation
    }

    /// Return the generics of the validated type as the header of an implementation spells them.
    pub(crate) fn impl_generics(&self) -> &ImplGenerics<'ir> {
        &self.impl_generics
    }

    /// Return the generics of the validated type as a type position spells them.
    pub(crate) fn ty_generics(&self) -> &TypeGenerics<'ir> {
        &self.ty_generics
    }

    /// Return the where clause of the validated type, which stays absent while the type declares none.
    pub(crate) fn where_clause(&self) -> Option<&'ir WhereClause> {
        self.where_clause
    }

    /// Return the generic arguments of the validated type as an expression position spells them.
    pub(crate) fn turbofish(&self) -> &Turbofish<'ir> {
        &self.turbofish
    }

    /// Return the validation error type of the validated type as every type position names it.
    /// The type carries the generic arguments of the validated type once a parameter reaches an error payload.
    pub(crate) fn error_type(&self) -> TokenStream {
        let error_ident = &self.intermediate_representation.error_ident;
        if !self.intermediate_representation.error_enum_is_generic {
            return quote! { #error_ident };
        }
        let ty_generics = &self.ty_generics;

        quote! { #error_ident #ty_generics }
    }

    /// Return the path that names a wrapper variant of the error enum as a function value.
    /// Such a position carries no return type to infer the generic arguments from, so the path spells them out.
    pub(crate) fn error_constructor_turbofish(&self) -> TokenStream {
        let error_ident = &self.intermediate_representation.error_ident;
        if !self.intermediate_representation.error_enum_is_generic {
            return quote! { #error_ident };
        }
        let turbofish = &self.turbofish;

        quote! { #error_ident #turbofish }
    }

    /// Generate the calls that run every final validation of the validated type on the draft named `draft_name`.
    /// Each call wraps its error inside the wrapper variant of the final validation.
    pub(crate) fn final_validation_calls(&self, draft_name: &TokenStream) -> TokenStream {
        let intermediate_representation = self.intermediate_representation;
        let type_ident = &intermediate_representation.ident;
        let turbofish = &self.turbofish;
        let error_constructor = self.error_constructor_turbofish();
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

    /// Generate the bound assertions of every nested type of the validated type against `trait_path`.
    /// The span of the nested type carries the diagnostic, so the compiler points at the field or at the variant payload.
    /// The generic parameters of the validated type reach the assertion function, which binds the ones the nested type names.
    fn nested_assertions(&self, trait_path: &TokenStream) -> TokenStream {
        let impl_generics = &self.impl_generics;
        let where_clause = self.where_clause;
        let assertion_allow = assertion_allow();
        let assertions = self.intermediate_representation.nested_types().map(|ty| {
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

    /// Generate the assertion that every final validation error of the validated type implements [`Error`](core::error::Error).
    /// The generated error enum reports such an error as its source, which needs the trait.
    /// The span of the error type carries the diagnostic, so the compiler points at the attribute.
    /// The generic parameters of the validated type reach the assertion function, which binds the ones the error type names.
    fn final_validation_error_assertions(&self) -> TokenStream {
        let impl_generics = &self.impl_generics;
        let where_clause = self.where_clause;
        let assertion_allow = assertion_allow();
        let assertions = self
            .intermediate_representation
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

    /// Generate the assertion that the validated type implements `Clone`, which every setter needs.
    /// The assertion function carries the generics of the validated type, so the assertion holds under its own where clause.
    fn clone_assertion(&self) -> TokenStream {
        let type_ident = &self.intermediate_representation.ident;
        let impl_generics = &self.impl_generics;
        let ty_generics = &self.ty_generics;
        let where_clause = self.where_clause;
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
}

/// Generate every item of the `Validate` derive of `intermediate_representation`.
/// The bound assertions come first, so their targeted diagnostic reaches the reader before the diagnostics of the generated items.
pub(crate) fn expand_validate(
    intermediate_representation: &TypeIntermediateRepresentation,
) -> TokenStream {
    let context = ExpansionContext::new(intermediate_representation);
    let assertions = context.nested_assertions(&validate_trait());
    let error_assertions = context.final_validation_error_assertions();
    let field_enum = field_enum::expand(&context);
    let error_enum = error_enum::expand(&context);
    let draft = draft::expand(&context);
    let validators = validators::expand(&context);
    let construction = construction::expand(&context);

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
    let context = ExpansionContext::new(intermediate_representation);
    let assertions = context.nested_assertions(&patch_trait());
    let clone_assertion = context.clone_assertion();
    let items = patch::expand(&context);

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

/// Build the attribute that lets an assertion function stay uncalled.
/// The three assertion shells share it, because a caller that never calls the function must see no warning.
/// An `expect` attribute would warn on its own every time the lint of the generated code stays quiet.
fn assertion_allow() -> TokenStream {
    quote! {
        #[allow(dead_code, reason = "the assertion function is type checked and never called")]
    }
}
