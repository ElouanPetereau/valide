//! Generic validated types whose bounds come from a local trait.
//!
//! The newtype carries a defaulted parameter and a where clause, which the generated draft mirrors.
//! The range of its field reads the two bounds from the parameter itself,
//! because a floating point literal would ask the parameter for a comparison between two precisions.

use valide::{Patch as _, Validate as _};

/// Bounds of the unit interval and the finite check at the precision of the implementor.
trait Bounded01 {
    /// Lower bound of the unit interval.
    const ZERO: Self;
    /// Upper bound of the unit interval.
    const ONE: Self;

    /// Whether the value is a finite number.
    fn is_finite(&self) -> bool;
}

impl Bounded01 for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;

    fn is_finite(&self) -> bool {
        // The inherent method needs its explicit path, because the trait method would call itself
        f32::is_finite(*self)
    }
}

impl Bounded01 for f64 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;

    fn is_finite(&self) -> bool {
        f64::is_finite(*self)
    }
}

/// Fraction bounded to the unit interval of its own precision.
#[derive(Clone, valide_derive::Validate, valide_derive::Patch)]
struct Fraction<Number: Bounded01 = f64>(
    /// The fraction itself.
    #[validate(range(Number::ZERO..=Number::ONE))]
    Number,
)
where
    Number: Clone + PartialOrd;

/// Reading whose measurement carries the precision of the parameter.
#[derive(Clone, valide_derive::Validate, valide_derive::Patch)]
struct Reading<Number>
where
    Number: Bounded01 + Clone,
{
    /// Measured value.
    #[validate(finite)]
    measurement: Number,
    /// Sensor that produced the reading, which takes part in no validation.
    #[validate(skip)]
    sensor: u8,
}

fn main() {
    // The defaulted parameter names the draft and the validated type without an argument
    let default_draft: FractionDraft = FractionDraft(0.5);
    let default_fraction: Fraction =
        Fraction::new(default_draft).expect("0.5 is inside the unit interval");
    assert!(
        (*default_fraction.value() - 0.5_f64).abs() < f64::EPSILON,
        "The defaulted parameter must give the double precision"
    );

    let mut single =
        Fraction::<f32>::new(FractionDraft(0.25)).expect("0.25 is inside the unit interval");
    assert!(
        (*single.value() - 0.25_f32).abs() < f32::EPSILON,
        "The getter of a bare parameter must return the built value"
    );

    assert_eq!(
        Fraction::<f32>::new(FractionDraft(1.5)).err(),
        Some(FractionValidationError::OutOfRange {
            field: FractionField::Value,
            range: "[Number::ZERO, Number::ONE]",
        }),
        "A value above the range must be rejected at the single precision"
    );
    assert_eq!(
        Fraction::<f64>::new(FractionDraft(-0.5)).err(),
        Some(FractionValidationError::OutOfRange {
            field: FractionField::Value,
            range: "[Number::ZERO, Number::ONE]",
        }),
        "A value below the range must be rejected at the double precision"
    );

    assert!(
        single.set_value(0.75).is_ok(),
        "A valid patch must be accepted"
    );
    assert!(
        single.set_value(2.0).is_err(),
        "An invalid patch must be rejected"
    );
    assert!(
        (*single.value() - 0.75_f32).abs() < f32::EPSILON,
        "A rejected patch must leave the value untouched"
    );

    let restored = Fraction::<f32>::from_draft(single.to_draft()).expect("The draft stays valid");
    assert!(
        (*restored.value() - 0.75_f32).abs() < f32::EPSILON,
        "The draft round trip must keep the value"
    );

    let mut reading = Reading::<f64>::new(ReadingDraft {
        measurement: 1.5,
        sensor: 3,
    })
    .expect("1.5 is a finite number");
    assert_eq!(
        reading.sensor(),
        3,
        "A skipped field must be passed through"
    );
    reading.set_sensor(4);
    assert_eq!(
        reading.sensor(),
        4,
        "The setter of a skipped field must store the value"
    );

    assert!(
        reading.set_measurement(2.5).is_ok(),
        "A valid patch of a finite field must be accepted"
    );
    assert!(
        reading.set_measurement(f64::NAN).is_err(),
        "A not a number patch must be rejected"
    );
    assert!(
        (*reading.measurement() - 2.5_f64).abs() < f64::EPSILON,
        "A rejected patch must leave the measurement untouched"
    );

    assert_eq!(
        Reading::<f32>::new(ReadingDraft {
            measurement: f32::NAN,
            sensor: 0,
        })
        .err(),
        Some(ReadingValidationError::NotFinite {
            field: ReadingField::Measurement,
        }),
        "A not a number value must be rejected at the single precision"
    );
    assert_eq!(
        Reading::<f64>::new(ReadingDraft {
            measurement: f64::INFINITY,
            sensor: 0,
        })
        .err(),
        Some(ReadingValidationError::NotFinite {
            field: ReadingField::Measurement,
        }),
        "An infinite value must be rejected at the double precision"
    );
}
