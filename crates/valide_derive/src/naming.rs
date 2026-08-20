//! Naming rules of every generated item.
//!
//! The module converts snake case field names to their Pascal case field enum variants.
//! It derives the wrapper variant names of the nested errors and of the final validations.
//! It also detects the collisions between the derived names.
//!
//! A name that cannot build an identifier produces an error, because the identifier constructor panics on an invalid spelling.

use proc_macro2::{Ident, Span};
use syn::{Error, Result, parse_str};

/// Suffix of the generated draft type.
pub(crate) const DRAFT_SUFFIX: &str = "Draft";
/// Suffix of the generated field enum.
pub(crate) const FIELD_ENUM_SUFFIX: &str = "Field";
/// Suffix of the generated validation error enum and of every one of its wrapper variants.
pub(crate) const VALIDATION_ERROR_SUFFIX: &str = "ValidationError";
/// Prefix that the generator removes from a final validation name to build its wrapper variant.
const VALIDATION_FUNCTION_PREFIX: &str = "validate_";
/// Prefix of a raw identifier, which no derived name carries.
const RAW_IDENT_PREFIX: &str = "r#";

/// Return the given `name` without the prefix of a raw identifier.
/// Every derived name comes from a plain name, so `r#type` derives the same names as `type`.
pub(crate) fn plain_name(name: &str) -> &str {
    name.strip_prefix(RAW_IDENT_PREFIX).unwrap_or(name)
}

/// Convert the given `snake_case` name to its Pascal case spelling.
pub(crate) fn to_pascal_case(snake_case: &str) -> String {
    let mut pascal_case = String::new();
    for word in plain_name(snake_case).split('_') {
        let mut characters = word.chars();
        if let Some(first_character) = characters.next() {
            pascal_case.extend(first_character.to_uppercase());
            pascal_case.push_str(characters.as_str());
        }
    }

    pascal_case
}

/// Build the identifier `base` plus `suffix`, with the span of `base`.
pub(crate) fn suffixed_ident(base: &Ident, suffix: &str) -> Ident {
    let name = base.to_string();

    Ident::new(&format!("{}{suffix}", plain_name(&name)), base.span())
}

/// Build the field enum variant of the field `logical_name`, with the span `span`.
/// Return an error when the name builds no identifier.
pub(crate) fn field_variant(logical_name: &str, span: Span) -> Result<Ident> {
    pascal_case_ident(logical_name, "", span)
}

/// Build the wrapper variant of the nested field `logical_name`, with the span `span`.
/// Return an error when the name builds no identifier.
pub(crate) fn nested_wrapper_variant(logical_name: &str, span: Span) -> Result<Ident> {
    pascal_case_ident(logical_name, VALIDATION_ERROR_SUFFIX, span)
}

/// Build the wrapper variant of the final validation `function_name`, with the span `span`.
/// Return an error when the name builds no identifier.
pub(crate) fn final_validation_wrapper_variant(function_name: &str, span: Span) -> Result<Ident> {
    let plain_function_name = plain_name(function_name);
    let stripped_name = plain_function_name
        .strip_prefix(VALIDATION_FUNCTION_PREFIX)
        .unwrap_or(plain_function_name);

    pascal_case_ident(stripped_name, VALIDATION_ERROR_SUFFIX, span)
}

/// Build the Pascal case spelling of `name` plus `suffix`, with the span `span`.
/// Return an error when the Pascal case spelling of `name` builds no identifier,
/// which happens when the spelling is empty or when it starts with a digit.
fn pascal_case_ident(name: &str, suffix: &str, span: Span) -> Result<Ident> {
    let pascal_case = to_pascal_case(name);
    // The identifier constructor panics on an invalid spelling, so the parser checks it first
    if parse_str::<Ident>(&pascal_case).is_err() {
        return Err(Error::new(
            span,
            format!(
                "the derive cannot build an identifier from the `{name}` name, which gives the \
                 invalid Pascal case spelling `{pascal_case}`"
            ),
        ));
    }

    Ok(Ident::new(&format!("{pascal_case}{suffix}"), span))
}

/// Return the first identifier of `variants` that collides with an earlier one.
/// The earlier identifier comes with it.
/// Return `None` when every identifier is distinct.
pub(crate) fn first_collision(variants: &[Ident]) -> Option<(&Ident, &Ident)> {
    for (position, variant) in variants.iter().enumerate() {
        for earlier_variant in variants.iter().take(position) {
            if earlier_variant == variant {
                return Some((earlier_variant, variant));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use proc_macro2::{Ident, Span};
    use syn::Result;

    use crate::naming::{
        DRAFT_SUFFIX, field_variant, final_validation_wrapper_variant, first_collision,
        nested_wrapper_variant, suffixed_ident, to_pascal_case,
    };

    /// Build an identifier from `name`, with the call site span.
    fn ident(name: &str) -> Ident {
        Ident::new(name, Span::call_site())
    }

    /// Return the name of the variant that `built_variant` carries.
    fn variant_name(built_variant: Result<Ident>) -> String {
        built_variant
            .expect("the tested name must build a variant")
            .to_string()
    }

    #[test]
    fn pascal_case_of_a_single_word() {
        assert_eq!(
            to_pascal_case("mass"),
            "Mass",
            "A single word must only be capitalized"
        );
    }

    #[test]
    fn pascal_case_of_several_words() {
        assert_eq!(
            to_pascal_case("sun_shadow_fraction"),
            "SunShadowFraction",
            "Every underscore separated word must be capitalized and joined"
        );
    }

    #[test]
    fn pascal_case_of_a_two_letter_word() {
        assert_eq!(
            to_pascal_case("xx"),
            "Xx",
            "Only the first letter of a word must be capitalized"
        );
        assert_eq!(
            to_pascal_case("r#type"),
            "Type",
            "The prefix of a raw identifier must not reach the Pascal case spelling"
        );
    }

    #[test]
    fn suffixed_ident_appends_the_suffix() {
        assert_eq!(
            suffixed_ident(&ident("InertiaMatrixSerializable"), DRAFT_SUFFIX).to_string(),
            "InertiaMatrixSerializableDraft",
            "The draft identifier must be the type name followed by the draft suffix"
        );
    }

    #[test]
    fn field_variant_of_a_newtype_value() {
        assert_eq!(
            variant_name(field_variant("value", Span::call_site())),
            "Value",
            "The logical name of a newtype field must give the Value variant"
        );
        assert_eq!(
            variant_name(field_variant("r#type", Span::call_site())),
            "Type",
            "A raw identifier field must give the variant of its plain name"
        );
        assert!(
            field_variant("_1", Span::call_site()).is_err(),
            "A name whose Pascal case spelling starts with a digit must be rejected"
        );
        assert!(
            field_variant("__", Span::call_site()).is_err(),
            "A name with an empty Pascal case spelling must be rejected"
        );
    }

    #[test]
    fn nested_wrapper_variant_matches_the_model() {
        assert_eq!(
            variant_name(nested_wrapper_variant("inertia_matrix", Span::call_site())),
            "InertiaMatrixValidationError",
            "A nested field must give its Pascal case name followed by the error suffix"
        );
        assert_eq!(
            variant_name(nested_wrapper_variant(
                "sun_shadow_fraction",
                Span::call_site()
            )),
            "SunShadowFractionValidationError",
            "A nested field must give its Pascal case name followed by the error suffix"
        );
    }

    #[test]
    fn final_validation_wrapper_variant_strips_the_prefix() {
        assert_eq!(
            variant_name(final_validation_wrapper_variant(
                "validate_mass_sum",
                Span::call_site()
            )),
            "MassSumValidationError",
            "The validate prefix must be stripped before the Pascal case conversion"
        );
        assert_eq!(
            variant_name(final_validation_wrapper_variant(
                "validate_realizability",
                Span::call_site()
            )),
            "RealizabilityValidationError",
            "The validate prefix must be stripped before the Pascal case conversion"
        );
    }

    #[test]
    fn final_validation_wrapper_variant_without_the_prefix() {
        assert_eq!(
            variant_name(final_validation_wrapper_variant(
                "check_masses",
                Span::call_site()
            )),
            "CheckMassesValidationError",
            "A function name without the validate prefix must be converted as it is"
        );
    }

    #[test]
    fn first_collision_finds_the_repeated_identifier() {
        let variants = [ident("MassValidationError"), ident("MassValidationError")];
        let collision = first_collision(&variants);

        assert!(
            collision.is_some(),
            "Two identical variants must be reported as a collision"
        );
    }

    #[test]
    fn first_collision_accepts_distinct_identifiers() {
        let variants = [ident("MassValidationError"), ident("SailValidationError")];
        let collision = first_collision(&variants);

        assert!(
            collision.is_none(),
            "Distinct variants must not be reported as a collision"
        );
    }
}
