//! A derive on a union, which the macros do not support.

#[derive(valide_derive::Validate)]
union Number {
    floating: f64,
    integer: u64,
}

fn main() {}
