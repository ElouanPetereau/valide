//! A field that carries the `skip` marker next to a `range` marker.

#[derive(valide_derive::Validate)]
struct Fraction {
    #[validate(skip, range(0.0..=1.0))]
    value: f64,
}

fn main() {}
