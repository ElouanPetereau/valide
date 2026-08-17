//! A `range` marker with three arguments.

#[derive(valide_derive::Validate)]
struct Fraction {
    #[validate(range(Bound::Included(0.0), Bound::Excluded(1.0), Bound::Unbounded))]
    value: f64,
}

fn main() {}
