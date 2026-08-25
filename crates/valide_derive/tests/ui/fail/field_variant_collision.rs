//! Two finite fields whose names give the same variant of the generated error enum.

#![allow(non_snake_case)]

#[derive(valide_derive::Validate)]
struct Spacecraft {
    #[validate(finite)]
    sun_shadow: f64,
    #[validate(finite)]
    sunShadow: f64,
}

fn main() {}
