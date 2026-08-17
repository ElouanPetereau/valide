//! A `range` marker whose single argument is not a range expression.

#[derive(valide_derive::Validate)]
struct Fraction {
    #[validate(range(1.0))]
    value: f64,
}

fn main() {}
