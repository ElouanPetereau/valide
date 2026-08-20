//! A variant payload that carries a `range` marker, which only a struct field accepts.

#[derive(valide_derive::Validate)]
enum Command {
    Halt,
    Extend(#[validate(range(0.0..=1.0))] f64),
}

fn main() {}
