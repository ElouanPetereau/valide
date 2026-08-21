//! A custom marker without its `error` argument.

#[derive(valide_derive::Validate)]
struct Fraction {
    #[validate(custom(check_value))]
    value: f64,
}

fn main() {}
