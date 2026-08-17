//! A field with two `validate` attributes.

#[derive(valide_derive::Validate)]
struct Fraction {
    #[validate(finite)]
    #[validate(finite)]
    value: f64,
}

fn main() {}
