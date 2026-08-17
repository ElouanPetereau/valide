//! Two passthrough attributes that carry no payload in parentheses.

#[derive(valide_derive::Validate)]
#[draft_attr]
#[draft_attr = "derive(Debug)"]
struct Fraction {
    #[validate(range(0.0..=1.0))]
    value: f64,
}

fn main() {}
