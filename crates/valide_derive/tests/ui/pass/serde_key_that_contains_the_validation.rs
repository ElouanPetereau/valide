//! A serde attribute whose value merely contains the text of the deserialization validation.
//!
//! The draft of the outer type holds the draft of its nested field, which derives no serde trait.
//! The fixture therefore fails to compile as soon as the outer draft mirrors a validation that the outer type does not carry.

use serde::Serialize;

/// Inner type that carries no deserialization validation.
#[derive(Serialize, valide_derive::Validate)]
struct Inner {
    /// The inner value.
    #[validate(finite)]
    value: f64,
}

/// Outer type whose serde attribute only contains the text of the validation.
#[derive(Serialize, valide_derive::Validate)]
#[serde(rename = "thing_try_from_disk")]
struct Outer {
    /// The inner value holder.
    #[validate(nested)]
    inner: Inner,
}

fn main() {
    let draft = OuterDraft {
        inner: InnerDraft { value: 1.0 },
    };

    assert!(
        Outer::new(draft).is_ok(),
        "A serde key that contains the validation text must not change the validation"
    );
}
