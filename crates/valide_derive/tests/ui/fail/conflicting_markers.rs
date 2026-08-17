//! A field with two markers inside one `validate` attribute.

#[derive(valide_derive::Validate)]
struct Fraction {
    #[validate(finite, nested)]
    value: f64,
}

fn main() {}
