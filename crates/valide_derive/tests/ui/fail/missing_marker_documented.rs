//! A documented field without a marker, whose diagnostic must underline the name alone.

#[derive(valide_derive::Validate)]
struct Fraction {
    /// The fraction itself.
    value: f64,
}

fn main() {}
