//! A `validate` attribute on the type instead of a field.

#[derive(valide_derive::Validate)]
#[validate(finite)]
struct Fraction {
    #[validate(finite)]
    value: f64,
}

fn main() {}
