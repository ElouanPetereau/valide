//! Two fields whose names give the same field enum variant.

#![allow(non_snake_case)]

#[derive(valide_derive::Validate)]
struct Spacecraft {
    #[validate(finite)]
    sun_shadow: f64,
    #[validate(finite)]
    sunShadow: f64,
}

fn main() {}
