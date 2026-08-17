//! Field names whose Pascal case spelling builds no identifier.

#[derive(valide_derive::Validate)]
struct Readings {
    #[validate(finite)]
    _1: f64,
    #[validate(finite)]
    __: f64,
}

fn main() {}
