//! A final validation and a nested field that generate the same wrapper variant.

#[derive(valide_derive::Validate)]
#[final_validation(validate_sun_shadow_fraction, error = f64)]
struct Spacecraft {
    #[validate(nested)]
    sun_shadow_fraction: f64,
}

fn main() {}
