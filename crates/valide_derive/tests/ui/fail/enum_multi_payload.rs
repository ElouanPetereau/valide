//! An enum variant that carries more than one payload.

#[derive(valide_derive::Validate)]
enum Command {
    Halt,
    Extend(f64, f64),
}

fn main() {}
