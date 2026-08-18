//! Two generic parameters where only one of them reaches an error payload.
//!
//! The nested field carries the `Number` parameter into the generated error enum of the outer type.
//! The `Other` parameter reaches no error payload, so the generated enum would carry an unused parameter,which the caller cannot fix.

#[derive(valide_derive::Validate)]
struct Inner<Number>(#[validate(skip)] Number);

#[derive(valide_derive::Validate)]
struct Wrapper<Number, Other> {
    #[validate(nested)]
    inner: Inner<Number>,
    #[validate(skip)]
    other: Other,
}

fn main() {}
