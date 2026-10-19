//! A `range` marker on an `Option` of an `Option`, whose check reads one `Option` level only.

#[derive(valide_derive::Validate)]
struct Fraction {
    #[validate(range(0.0..=1.0))]
    value: Option<Option<f64>>,
}

fn main() {}
