//! A generic validated type whose deserialization validation names the generics of its draft.
//!
//! The serde attribute of the validated type carries the parameter inside its own text,
//! and the generated draft mirrors the two serde derives.

use serde::{Deserialize, Serialize};

/// Bounds of the unit interval at the precision of the implementor.
trait UnitInterval {
    /// Lower bound of the unit interval.
    const ZERO: Self;
    /// Upper bound of the unit interval.
    const ONE: Self;
}

impl UnitInterval for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
}

impl UnitInterval for f64 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
}

/// Scale factor bounded to the unit interval of its own precision.
#[derive(Debug, Serialize, Deserialize, valide_derive::Validate)]
#[serde(try_from = "ScaleDraft<Number>")]
struct Scale<Number>(
    /// The scale factor itself.
    #[validate(range(Number::ZERO..=Number::ONE))]
    Number,
)
where
    Number: UnitInterval + PartialOrd;

fn main() {
    let single: Scale<f32> = serde_json::from_str("0.5").expect("0.5 is inside the unit interval");
    assert!(
        (*single.value() - 0.5_f32).abs() < f32::EPSILON,
        "The validation must accept a valid document at the single precision"
    );

    let double: Scale<f64> =
        serde_json::from_str("0.25").expect("0.25 is inside the unit interval");
    assert!(
        (*double.value() - 0.25_f64).abs() < f64::EPSILON,
        "The validation must accept a valid document at the double precision"
    );

    let document = serde_json::to_string(&double).expect("A scale serializes as its own value");
    assert_eq!(
        document, "0.25",
        "The wire format must stay the value of the newtype"
    );

    let rejection =
        serde_json::from_str::<Scale<f64>>("1.5").expect_err("1.5 is outside the unit interval");
    assert!(
        rejection
            .to_string()
            .contains("must be within the range [Number::ZERO, Number::ONE]"),
        "The deserialization must report the validation error, and it says: {rejection}"
    );
}
