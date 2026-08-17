//! A bound pair with a variant that `Bound` does not have.

#[derive(valide_derive::Validate)]
struct Fraction {
    #[validate(range(Bound::Included(0.0), Bound::Beyond(1.0)))]
    value: f64,
}

fn main() {}
