//! A final validation whose second argument is not `error`.

#[derive(valide_derive::Validate)]
#[final_validation(validate_fraction, oops = FractionValidationError)]
struct Fraction {
    #[validate(finite)]
    value: f64,
}

fn main() {}
