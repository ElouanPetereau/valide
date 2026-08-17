//! A derive on a generic struct, which the macros do not support.

#[derive(valide_derive::Validate)]
struct Fraction<Number> {
    #[validate(finite)]
    value: Number,
}

fn main() {}
