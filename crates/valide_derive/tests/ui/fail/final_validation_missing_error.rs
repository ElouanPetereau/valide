//! A final validation without its `error` argument.

#[derive(valide_derive::Validate)]
#[final_validation(validate_fraction)]
struct Fraction {
    #[validate(finite)]
    value: f64,
}

fn main() {}
