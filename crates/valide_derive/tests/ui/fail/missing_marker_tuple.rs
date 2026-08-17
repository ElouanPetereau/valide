//! A newtype field without a marker.

#[derive(valide_derive::Validate)]
struct Fraction(f64);

fn main() {}
