//! A generic enum whose nested payload carries the parameter of the validated enum.
//!
//! The parameter reaches the error payload through the declared type of the payload,
//! so the generated error enum carries it and the wrapper variant works at every precision.

use core::fmt::{Debug, Display};

use valide::{Patch as _, Validate as _};

/// Finite check of a measurement at the precision of the implementor.
trait Precision: Copy {
    /// Whether the value is a finite number.
    fn is_finite(&self) -> bool;
}

impl Precision for f32 {
    fn is_finite(&self) -> bool {
        // The inherent method needs its explicit path, because the trait method would call itself
        f32::is_finite(*self)
    }
}

impl Precision for f64 {
    fn is_finite(&self) -> bool {
        f64::is_finite(*self)
    }
}

/// Measurement that must be a finite number at its own precision.
#[derive(Clone, PartialEq, Debug, valide_derive::Validate, valide_derive::Patch)]
struct Measurement<Number: Precision>(
    /// The measured value itself.
    #[validate(finite)]
    Number,
);

/// Sample that a sensor reported at its own precision.
#[derive(Clone, PartialEq, Debug, valide_derive::Validate, valide_derive::Patch)]
enum Sample<Number>
where
    Number: Precision + Debug + Display + 'static,
{
    /// The sensor reported no value at all.
    Missing,
    /// Identifier of the sensor, which takes part in no validation.
    Sensor(#[validate(skip)] u8),
    /// Measurement that the sensor reported.
    Measured(#[validate(nested)] Measurement<Number>),
}

fn main() {
    let double = Sample::<f64>::new(SampleDraft::Measured(MeasurementDraft(2.5)))
        .expect("2.5 is a finite measurement");
    assert_eq!(
        double,
        Sample::Measured(
            Measurement::new(MeasurementDraft(2.5_f64)).expect("2.5 is a finite measurement")
        ),
        "A valid nested payload must build the sample at the double precision"
    );

    assert_eq!(
        Sample::<f64>::new(SampleDraft::Sensor(7)).expect("A skipped payload is always valid"),
        Sample::Sensor(7),
        "A skipped payload must be passed through at the double precision"
    );

    // The generic wrapper variant of the enum holds the error of the nested measurement
    assert_eq!(
        Sample::<f64>::new(SampleDraft::Measured(MeasurementDraft(f64::NAN))).err(),
        Some(SampleValidationError::MeasuredValidationError(
            MeasurementValidationError::NotFinite {
                field: MeasurementField::Value,
            }
        )),
        "A not a number measurement must be rejected at the double precision"
    );

    let single = Sample::<f32>::new(SampleDraft::Missing).expect("A unit variant is always valid");
    assert_eq!(
        single,
        Sample::Missing,
        "A unit variant must build the matching variant at the single precision"
    );

    assert_eq!(
        Sample::<f32>::new(SampleDraft::Measured(MeasurementDraft(f32::INFINITY))).err(),
        Some(SampleValidationError::MeasuredValidationError(
            MeasurementValidationError::NotFinite {
                field: MeasurementField::Value,
            }
        )),
        "An infinite measurement must be rejected at the single precision"
    );

    let restored = Sample::<f32>::from_draft(
        Sample::<f32>::new(SampleDraft::Measured(MeasurementDraft(1.5_f32)))
            .expect("1.5 is a finite measurement")
            .to_draft(),
    )
    .expect("The draft stays valid");
    assert_eq!(
        restored,
        Sample::Measured(
            Measurement::new(MeasurementDraft(1.5_f32)).expect("1.5 is a finite measurement")
        ),
        "The draft round trip must keep the variant of the single precision sample"
    );
}
