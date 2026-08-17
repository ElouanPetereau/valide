//! A newtype with a range marker and a deserialization validation.

use serde::{Deserialize, Serialize};
use valide::{Patch as _, Validate as _};

/// Fraction bounded to [0.0, 1.0].
#[derive(Clone, Serialize, Deserialize, valide_derive::Validate, valide_derive::Patch)]
#[serde(try_from = "FractionDraft")]
struct Fraction(
    /// The fraction itself.
    #[validate(range(0.0..=1.0))]
    f64,
);

fn main() {
    let mut fraction = Fraction::new(FractionDraft(0.25)).expect("0.25 is inside the range");
    assert!(
        (fraction.value() - 0.25).abs() < f64::EPSILON,
        "The getter must return the built value"
    );

    assert_eq!(
        Fraction::new(FractionDraft(1.5)).err(),
        Some(FractionValidationError::OutOfRange {
            field: FractionField::Value,
            range: "[0.0, 1.0]",
        }),
        "A value above the range must be rejected"
    );

    assert!(
        fraction.set_value(0.75).is_ok(),
        "A valid patch must be accepted"
    );
    assert!(
        fraction.set_value(2.0).is_err(),
        "An invalid patch must be rejected"
    );
    assert!(
        (fraction.value() - 0.75).abs() < f64::EPSILON,
        "A rejected patch must leave the value untouched"
    );

    let restored = Fraction::from_draft(fraction.to_draft()).expect("The draft stays valid");
    assert!(
        (restored.value() - 0.75).abs() < f64::EPSILON,
        "The draft round trip must keep the value"
    );

    let deserialized: Fraction = serde_json::from_str("0.5").expect("0.5 is inside the range");
    assert!(
        (deserialized.value() - 0.5).abs() < f64::EPSILON,
        "The validation must accept a valid document"
    );
    assert!(
        serde_json::from_str::<Fraction>("1.5").is_err(),
        "The validation must reject an invalid document"
    );
}
