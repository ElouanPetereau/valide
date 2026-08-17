//! A marker attribute without parentheses, which carries no marker at all.

#[derive(valide_derive::Validate)]
struct Fraction {
    #[validate]
    value: f64,
}

fn main() {}
