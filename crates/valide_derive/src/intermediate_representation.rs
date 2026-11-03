//! Intermediate representation that the parsing stage and the expanders share.
//!
//! The representation describes the validated type, its fields or variants and the final validations.
//! Each field payload (for a struct) and each variant payload (for an enum) carries its validation rule.
//! The expanders never look at a syntax tree again.

use proc_macro2::{Span, TokenStream};
use syn::{Attribute, Generics, Ident, Member, Path, Type, Visibility};

/// Names of the primitive types that a getter returns by value.
/// A derive cannot detect `Copy`, so the generator compares the token of the field type with this list
/// and returns a reference to every other type.
const BY_VALUE_TYPES: &[&str] = &[
    "bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32",
    "u64", "u128", "usize",
];
/// Prefix of the identifier of a field validator.
const VALIDATOR_PREFIX: &str = "validate_";
/// Prefix of the identifier of a setter.
const SETTER_PREFIX: &str = "set_";
/// Prefix of the identifier of the new value that a setter takes.
const NEW_VALUE_PREFIX: &str = "new_";

/// Shape of a validated type, which drives the field access of the generated code.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Shape {
    /// Struct with named fields.
    Named,
    /// Tuple struct with exactly one unnamed field.
    Newtype,
    /// Enum whose variants are unit variants, tuple variants with exactly one payload or struct variants.
    Enum,
}

/// Validation rule of a single struct field.
pub(crate) enum FieldRule {
    /// The value must be inside a range.
    Range {
        /// Expression of the lower bound of the range, an absolute [`Bound`](core::ops::Bound) variant.
        lower: TokenStream,
        /// Expression of the upper bound of the range, an absolute [`Bound`](core::ops::Bound) variant.
        upper: TokenStream,
        /// Variant of the generated error enum that carries the two evaluated bounds and the rejected value.
        error_variant: Ident,
        /// Payload type of the field when its declared type is written as an `Option`.
        /// The check then reads the payload under the `Some` pattern and the error variant carries this type.
        inner_ty: Option<Type>,
    },
    /// The value must be a finite number.
    Finite {
        /// Variant of the generated error enum that carries the rejected value.
        error_variant: Ident,
        /// Payload type of the field when its declared type is written as an `Option`.
        /// The check then reads the payload under the `Some` pattern and the error variant carries this type.
        inner_ty: Option<Type>,
    },
    /// The type of the field validates the value with its own `Validate` implementation.
    Nested {
        /// Wrapper variant that carries the error of the nested type.
        wrapper_variant: Ident,
    },
    /// A function of the validated type validates the value.
    Custom {
        /// Function that the generated field validator calls on a reference to the value.
        fn_ident: Ident,
        /// Error type that the function returns.
        error_ty: Path,
        /// Wrapper variant that carries the error of the function.
        wrapper_variant: Ident,
    },
    /// The value takes part in no validation at all.
    Skip,
}

/// One field of a validated struct.
pub(crate) struct FieldIntermediateRepresentation {
    /// Access to the field on `self`, the name of a named field or the index of a newtype.
    pub(crate) member: Member,
    /// Name that the generated identifiers use, `value` for the field of a newtype.
    pub(crate) logical_name: String,
    /// Declared type of the field.
    pub(crate) ty: Type,
    /// Documentation attributes of the field, which the generator clones onto the draft field.
    pub(crate) docs: Vec<Attribute>,
    /// Payloads that the generator re-emits as attributes of the draft field.
    /// A serde attribute of the field lands here whole, a `draft_attr` attribute lands here without its own name.
    pub(crate) passthrough: Vec<TokenStream>,
    /// Validation rule of the field.
    pub(crate) rule: FieldRule,
}

impl FieldIntermediateRepresentation {
    /// Return the error enum variant that belongs to the field alone, absent while the rule of the field wraps an error or declares no rule.
    /// A range field and a finite field each carry one, because each rejection comes from the field itself.
    pub(crate) fn own_variant(&self) -> Option<&Ident> {
        match &self.rule {
            FieldRule::Range { error_variant, .. } | FieldRule::Finite { error_variant, .. } => {
                Some(error_variant)
            }
            FieldRule::Nested { .. } | FieldRule::Custom { .. } | FieldRule::Skip => None,
        }
    }

    /// Return the wrapper variant of the field, absent while the rule of the field reaches the generated error enum with no wrapper.
    /// A nested field and a custom field each carry one, and a range, a finite or a skip field carries none.
    pub(crate) fn wrapper_variant(&self) -> Option<&Ident> {
        match &self.rule {
            FieldRule::Nested { wrapper_variant }
            | FieldRule::Custom {
                wrapper_variant, ..
            } => Some(wrapper_variant),
            FieldRule::Range { .. } | FieldRule::Finite { .. } | FieldRule::Skip => None,
        }
    }

    /// Return the identifier of the getter of the field.
    /// A named field keeps its own identifier, so a raw identifier stays valid.
    /// The unnamed arm builds the constant logical name of a newtype field.
    pub(crate) fn getter_ident(&self) -> Ident {
        match &self.member {
            Member::Named(ident) => ident.clone(),
            Member::Unnamed(index) => Ident::new(&self.logical_name, index.span),
        }
    }

    /// Return the identifier of the field validator of the field.
    pub(crate) fn validator_ident(&self) -> Ident {
        self.prefixed_ident(VALIDATOR_PREFIX)
    }

    /// Return the identifier of the setter of the field.
    pub(crate) fn setter_ident(&self) -> Ident {
        self.prefixed_ident(SETTER_PREFIX)
    }

    /// Return the identifier of the new value that the setter of the field takes.
    pub(crate) fn new_value_ident(&self) -> Ident {
        self.prefixed_ident(NEW_VALUE_PREFIX)
    }

    /// Whether a getter of the field returns the value instead of a reference to it.
    pub(crate) fn is_returned_by_value(&self) -> bool {
        let Type::Path(path_type) = &self.ty else {
            return false;
        };
        if path_type.qself.is_some() {
            return false;
        }
        let Some(ident) = path_type.path.get_ident() else {
            return false;
        };

        BY_VALUE_TYPES.iter().any(|name| ident == name)
    }

    /// Return the payload type of the field when its range check or its finite check reads the payload of an `Option`.
    /// Return nothing for every other field, whose check reads the whole value.
    pub(crate) fn option_payload_ty(&self) -> Option<&Type> {
        match &self.rule {
            FieldRule::Range { inner_ty, .. } | FieldRule::Finite { inner_ty, .. } => {
                inner_ty.as_ref()
            }
            FieldRule::Nested { .. } | FieldRule::Custom { .. } | FieldRule::Skip => None,
        }
    }

    /// Return the type that the range check or the finite check of the field reads and that its error variant carries.
    /// The payload type of an `Option` field, or the declared type of every other field.
    pub(crate) fn checked_ty(&self) -> &Type {
        self.option_payload_ty().unwrap_or(&self.ty)
    }

    /// Build the identifier `prefix` plus the logical name of the field, with the span of the field.
    /// The parsing stage rejects a logical name that builds no identifier
    /// and it also removes the prefix of a raw identifier, so the built name is always valid.
    fn prefixed_ident(&self, prefix: &str) -> Ident {
        Ident::new(&format!("{prefix}{}", self.logical_name), self.span())
    }

    /// Return the span of the field, which every derived identifier of the field carries.
    fn span(&self) -> Span {
        match &self.member {
            Member::Named(ident) => ident.span(),
            Member::Unnamed(index) => index.span,
        }
    }
}

/// Validation rule of the payload of a variant.
/// A payload carries no range and no finite rule, because the enum adds no invariant of its own.
pub(crate) enum VariantRule {
    /// The type of the payload validates the value with its own `Validate` implementation.
    Nested {
        /// Wrapper variant that carries the error of the nested type.
        wrapper_variant: Ident,
    },
    /// The value takes part in no validation at all.
    Skip,
}

/// Payload of a tuple variant or of one named field of a struct variant of a validated enum.
pub(crate) struct Payload {
    /// Declared type of the payload.
    pub(crate) ty: Type,
    /// Validation rule of the payload.
    pub(crate) rule: VariantRule,
    /// Documentation attributes of the payload, which the generator clones onto the draft payload.
    pub(crate) docs: Vec<Attribute>,
    /// Payloads that the generator re-emits as attributes of the draft payload.
    /// A serde attribute of the payload lands here whole, a `draft_attr` attribute lands here without its own name.
    pub(crate) passthrough: Vec<TokenStream>,
}

impl Payload {
    /// Return the wrapper variant of the payload while its rule is nested.
    /// Return nothing for a skip payload, which reaches no error enum.
    pub(crate) fn nested_wrapper(&self) -> Option<&Ident> {
        match &self.rule {
            VariantRule::Nested { wrapper_variant } => Some(wrapper_variant),
            VariantRule::Skip => None,
        }
    }
}

/// One named field of a struct variant of a validated enum.
pub(crate) struct VariantFieldIntermediateRepresentation {
    /// Identifier of the field, which the draft field and every generated pattern reuse, so a raw identifier stays valid.
    pub(crate) ident: Ident,
    /// Name that the generated documentation and the wrapper variant use, the identifier without the prefix of a raw identifier.
    pub(crate) logical_name: String,
    /// Payload that the field holds.
    pub(crate) payload: Payload,
}

/// Content of one variant of a validated enum.
#[expect(
    clippy::large_enum_variant,
    reason = "The representation of a variant lives for one expansion only, so the size of the payload variant costs nothing"
)]
pub(crate) enum VariantKind {
    /// Variant that carries no payload.
    Unit,
    /// Tuple variant that carries exactly one payload.
    Payload(Payload),
    /// Struct variant that carries named fields, each with its own rule.
    /// The list stays empty for a variant written with empty braces, which the draft keeps distinct from a unit variant.
    Struct {
        /// Fields in declaration order.
        fields: Vec<VariantFieldIntermediateRepresentation>,
    },
}

/// One variant of a validated enum.
pub(crate) struct VariantIntermediateRepresentation {
    /// Identifier of the variant, which the draft and the error enum reuse.
    pub(crate) ident: Ident,
    /// Documentation attributes of the variant, which the generator clones onto the draft variant.
    pub(crate) docs: Vec<Attribute>,
    /// Payloads that the generator re-emits as attributes of the draft variant.
    /// A serde attribute of the variant lands here whole, a `draft_attr` attribute lands here without its own name.
    pub(crate) passthrough: Vec<TokenStream>,
    /// Content of the variant.
    pub(crate) kind: VariantKind,
}

impl VariantIntermediateRepresentation {
    /// Return the declared type and the wrapper variant of every nested payload of the variant, in declaration order.
    /// A tuple variant gives its single payload while it is nested and a struct variant gives each nested field,
    /// because only a nested payload reaches the generated error enum.
    pub(crate) fn nested_payloads(&self) -> impl Iterator<Item = (&Type, &Ident)> {
        let (tuple_payload, struct_fields): (
            Option<&Payload>,
            &[VariantFieldIntermediateRepresentation],
        ) = match &self.kind {
            VariantKind::Unit => (None, &[]),
            VariantKind::Payload(payload) => (Some(payload), &[]),
            VariantKind::Struct { fields } => (None, fields),
        };

        tuple_payload
            .into_iter()
            .chain(struct_fields.iter().map(|field| &field.payload))
            .filter_map(|payload| Some((&payload.ty, payload.nested_wrapper()?)))
    }
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
    /// Identifier of the generated validation error enum.
    pub(crate) error_ident: Ident,
    /// Whether a generic parameter of the validated type reaches a wrapped error type.
    /// The generated error enum then carries the generics of the validated type.
    pub(crate) error_enum_is_generic: bool,
    /// Shape of the validated type.
    pub(crate) shape: Shape,
    /// Fields in declaration order, empty for an enum.
    pub(crate) fields: Vec<FieldIntermediateRepresentation>,
    /// Variants in declaration order, empty for a struct.
    pub(crate) variants: Vec<VariantIntermediateRepresentation>,
    /// Final validation functions in attribute order.
    pub(crate) final_validations: Vec<FinalValidation>,
    /// Whether the generated draft must derive the serde traits.
    pub(crate) emit_draft_serde: bool,
    /// Payloads of the `draft_attr` attributes, which the generator re-emits on the draft verbatim.
    pub(crate) draft_passthrough: Vec<TokenStream>,
}

impl TypeIntermediateRepresentation {
    /// Return the error enum variant of every range field and of every finite field of the validated type, in declaration order.
    /// A range variant carries the two evaluated bounds of its own field and the rejected value, a finite variant carries the rejected value.
    pub(crate) fn own_variants(&self) -> impl Iterator<Item = &Ident> {
        self.fields
            .iter()
            .filter_map(FieldIntermediateRepresentation::own_variant)
    }

    /// Return the wrapper variants of the validated type, the nested variants first, then the final validations and the fields after.
    /// A nested field and a custom field share the field block, where both keep the declaration order of the fields.
    /// The order matches the order of the variants of the generated enum.
    /// A struct variant gives one wrapper per nested field, in field order.
    pub(crate) fn wrapper_variants(&self) -> impl Iterator<Item = &Ident> {
        let payload_variants = self.variants.iter().flat_map(|variant| {
            variant
                .nested_payloads()
                .map(|(_, wrapper_variant)| wrapper_variant)
        });
        let final_variants = self
            .final_validations
            .iter()
            .map(|final_validation| &final_validation.wrapper_variant);
        let field_variants = self
            .fields
            .iter()
            .filter_map(FieldIntermediateRepresentation::wrapper_variant);

        payload_variants.chain(final_variants).chain(field_variants)
    }

    /// Return the error type of every custom field of the validated type, in declaration order.
    /// The generated error enum wraps such an error and forwards the source that it reports.
    pub(crate) fn custom_error_types(&self) -> impl Iterator<Item = &Path> {
        self.fields.iter().filter_map(|field| match &field.rule {
            FieldRule::Custom { error_ty, .. } => Some(error_ty),
            FieldRule::Range { .. }
            | FieldRule::Finite { .. }
            | FieldRule::Nested { .. }
            | FieldRule::Skip => None,
        })
    }

    /// Return the declared type of every nested field and of every nested variant payload of the validated type.
    /// Such a type carries a validation of its own, which the generated code delegates to.
    pub(crate) fn nested_types(&self) -> impl Iterator<Item = &Type> {
        let field_types = self
            .fields
            .iter()
            .filter(|field| matches!(field.rule, FieldRule::Nested { .. }))
            .map(|field| &field.ty);
        let payload_types = self.variants.iter().flat_map(|variant| {
            variant
                .nested_payloads()
                .map(|(payload_type, _)| payload_type)
        });

        field_types.chain(payload_types)
    }
}
