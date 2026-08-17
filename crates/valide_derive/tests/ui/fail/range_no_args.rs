//! A `range` marker without an argument.

#[derive(valide_derive::Validate)]
struct Fraction {
    #[validate(range())]
    value: f64,
}

fn main() {}
