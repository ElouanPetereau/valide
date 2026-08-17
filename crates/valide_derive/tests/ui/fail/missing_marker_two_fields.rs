//! Two fields without a marker, which one compilation reports together.

#[derive(valide_derive::Validate)]
struct Masses {
    bus_mass: f64,
    sail_mass: f64,
}

fn main() {}
