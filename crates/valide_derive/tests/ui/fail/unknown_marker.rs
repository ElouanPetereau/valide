//! A field with a marker that the grammar does not have.

#[derive(valide_derive::Validate)]
struct Fraction {
    #[validate(positive)]
    value: f64,
}

fn main() {}
