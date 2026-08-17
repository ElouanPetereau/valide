//! A `final_validation` attribute on a field instead of the type.

#[derive(valide_derive::Validate)]
struct Fraction {
    #[final_validation(validate_fraction, error = FractionValidationError)]
    #[validate(finite)]
    value: f64,
}

fn main() {}
