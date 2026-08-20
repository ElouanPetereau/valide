//! A marker on a unit variant, which carries no payload to mark.

#[derive(valide_derive::Validate)]
enum Command {
    #[validate(skip)]
    Halt,
    Extend(#[validate(skip)] f64),
}

fn main() {}
