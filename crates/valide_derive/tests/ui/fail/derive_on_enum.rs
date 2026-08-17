//! A derive on an enum, which the macros do not support.

#[derive(valide_derive::Validate)]
enum CelestialBodyKind {
    Sun,
    Earth,
}

fn main() {}
