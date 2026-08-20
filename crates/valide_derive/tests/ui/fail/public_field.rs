//! A public field next to a private one, which pins the rejected field.

#[derive(valide_derive::Validate)]
struct Masses {
    #[validate(finite)]
    pub bus_mass: f64,
    #[validate(finite)]
    sail_mass: f64,
}

fn main() {}
