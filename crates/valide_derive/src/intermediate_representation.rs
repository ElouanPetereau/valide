//! Intermediate representation that the parsing stage and the expanders share.
//!
//! The representation describes the validated type, its fields and the final validations.
//! Each field carries its validation rule.
//! The expanders never look at a syntax tree again.

use proc_macro2::TokenStream;
use syn::{Attribute, Generics, Ident, Member, Path, Type, Visibility};

/// Shape of a validated type, which drives the field access of the generated code.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Shape {
    /// Struct with named fields.
    Named,
    /// Tuple struct with exactly one unnamed field.
    Newtype,
}

/// Validation rule of a single field.
pub(crate) enum Rule {
    /// The value must be inside a range.
    Range {
        /// Tokens of the range, which the generated check uses as its `RangeBounds` implementor.
        check_tokens: TokenStream,
        /// Range text that the generated error carries.
        text: String,
    },
    /// The value must be a finite number.
    Finite,
    /// The type of the field validates the value with its own `Validate` implementation.
    Nested {
        /// Wrapper variant that carries the error of the nested type.
        wrapper_variant: Ident,
    },
    /// The value takes part in no validation at all.
    Skip,
}

/// One field of a validated type.
pub(crate) struct FieldIntermediateRepresentation {
    /// Access to the field on `self`, the name of a named field or the index of a newtype.
    pub(crate) member: Member,
    /// Name that the generated identifiers use, `value` for the field of a newtype.
    pub(crate) logical_name: String,
    /// Variant of the field enum that names the field.
    /// Only a range or a finite field has one, because only those two report the field they hold.
    pub(crate) variant: Option<Ident>,
    /// Declared type of the field.
    pub(crate) ty: Type,
    /// Documentation attributes of the field, which the generator clones onto the draft field.
    pub(crate) docs: Vec<Attribute>,
    /// Payloads that the generator re-emits as attributes of the draft field.
    /// A serde attribute of the field lands here whole, a `draft_attr` attribute lands here without its own name.
    pub(crate) passthrough: Vec<TokenStream>,
    /// Validation rule of the field.
    pub(crate) rule: Rule,
}

/// One final validation function of a validated type.
pub(crate) struct FinalValidation {
    /// Function that the draft validation calls once every field validator passed.
    pub(crate) fn_ident: Ident,
    /// Error type that the function returns.
    pub(crate) error_ty: Path,
    /// Wrapper variant that carries the error of the function.
    pub(crate) wrapper_variant: Ident,
}

/// A validated type and everything the expanders need to generate its code.
pub(crate) struct TypeIntermediateRepresentation {
    /// Identifier of the validated type.
    pub(crate) ident: Ident,
    /// Visibility of the validated type, which every generated item shares.
    pub(crate) vis: Visibility,
    /// Generics of the validated type, copied verbatim from the derive input.
    /// Every generated item carries them, a declaration with its bounds and an implementation with its split spelling.
    pub(crate) generics: Generics,
    /// Identifier of the generated draft.
    pub(crate) draft_ident: Ident,
    /// Identifier of the generated field enum.
    pub(crate) field_enum_ident: Ident,
    /// Identifier of the generated validation error enum.
    pub(crate) error_ident: Ident,
    /// Whether a generic parameter of the validated type reaches a wrapped error type.
    /// The generated error enum then carries the generics of the validated type.
    pub(crate) error_enum_is_generic: bool,
    /// Shape of the validated type.
    pub(crate) shape: Shape,
    /// Fields in declaration order.
    pub(crate) fields: Vec<FieldIntermediateRepresentation>,
    /// Final validation functions in attribute order.
    pub(crate) final_validations: Vec<FinalValidation>,
    /// Whether the generated draft must derive the serde traits.
    pub(crate) emit_draft_serde: bool,
    /// Payloads of the `draft_attr` attributes, which the generator re-emits on the draft verbatim.
    pub(crate) draft_passthrough: Vec<TokenStream>,
}
