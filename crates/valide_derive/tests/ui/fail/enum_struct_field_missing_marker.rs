//! A struct variant field without a marker, which the grammar rejects like a payload without a marker.

#[derive(valide_derive::Validate)]
enum Command {
    Halt,
    Extend { fraction: f64 },
}

fn main() {}
