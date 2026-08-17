//! A derive on a unit struct, which has no field to validate.

#[derive(valide_derive::Validate)]
struct Fraction;

fn main() {}
