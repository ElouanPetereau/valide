//! An enum variant with named fields, which the macros do not support.

#[derive(valide_derive::Validate)]
enum Command {
    Halt,
    Extend { fraction: f64 },
}

fn main() {}
