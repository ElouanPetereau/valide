//! A generic composite whose generated error enum carries the parameter of the validated type.
//!
//! The parameter reaches the error payload twice, through the error of the nested newtype
//! and through the error type of the final validation, so the generated enum carries it.

use core::{
    error::Error,
    fmt::{Debug, Display, Formatter, Result as FormatResult},
};

use valide::{Patch as _, Validate as _};

/// Finite check and ceiling of a measurement at the precision of the implementor.
trait Precision: Copy + PartialOrd {
    /// Largest accepted measurement.
    const CEILING: Self;

    /// Whether the value is a finite number.
    fn is_finite(&self) -> bool;
}

impl Precision for f32 {
    const CEILING: Self = 100.0;

    fn is_finite(&self) -> bool {
        // The inherent method needs its explicit path, because the trait method would call itself
        f32::is_finite(*self)
    }
}

impl Precision for f64 {
    const CEILING: Self = 100.0;

    fn is_finite(&self) -> bool {
        f64::is_finite(*self)
    }
}

/// Error of the ceiling check of a [`Reading`], which carries the rejected measurement.
#[derive(Clone, PartialEq, Debug)]
struct AboveCeilingError<Number> {
    /// Measurement that sits above the ceiling of its own precision.
    measurement: Number,
}

impl<Number: Display> Display for AboveCeilingError<Number> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
        write!(
            formatter,
            "The measurement {} is above the ceiling",
            self.measurement
        )
    }
}

impl<Number: Debug + Display> Error for AboveCeilingError<Number> {}

/// Measurement that must be a finite number at its own precision.
#[derive(Clone, valide_derive::Validate, valide_derive::Patch)]
struct Measurement<Number: Precision>(
    /// The measured value itself.
    #[validate(finite)]
    Number,
);

/// Reading whose measurement must stay below the ceiling of its own precision.
#[derive(Clone, valide_derive::Validate, valide_derive::Patch)]
#[final_validation(validate_ceiling, error = AboveCeilingError<Number>)]
struct Reading<Number>
where
    Number: Precision + Debug + Display + 'static,
{
    /// Measurement of the reading.
    #[validate(nested)]
    measurement: Measurement<Number>,
    /// Calibration value of the sensor, which the final validation reads in no way.
    #[validate(finite)]
    calibration: Number,
    /// Sensor that produced the reading, which takes part in no validation.
    #[validate(skip)]
    sensor: u8,
}

impl<Number> Reading<Number>
where
    Number: Precision + Debug + Display + 'static,
{
    /// Check that the measurement of the given `draft` stays below the ceiling of its precision.
    fn validate_ceiling(draft: &ReadingDraft<Number>) -> Result<(), AboveCeilingError<Number>> {
        if draft.measurement.0 > Number::CEILING {
            return Err(AboveCeilingError {
                measurement: draft.measurement.0,
            });
        }

        Ok(())
    }
}

/// Build a reading draft from the given `measurement` and `calibration`, with a fixed sensor.
/// The draft carries the where clause of the reading, so the builder repeats it.
fn reading_draft<Number>(measurement: Number, calibration: Number) -> ReadingDraft<Number>
where
    Number: Precision + Debug + Display + 'static,
{
    ReadingDraft {
        measurement: MeasurementDraft(measurement),
        calibration,
        sensor: 7,
    }
}

fn main() {
    let mut double =
        Reading::<f64>::new(reading_draft(2.5, 1.0)).expect("2.5 is below the ceiling");
    assert!(
        (*double.measurement().value() - 2.5_f64).abs() < f64::EPSILON,
        "The valid draft must build the reading at the double precision"
    );
    assert_eq!(double.sensor(), 7, "A skipped field must be passed through");

    // The generated enum of the outer type wraps the error of the nested newtype
    assert_eq!(
        Reading::<f64>::new(reading_draft(f64::NAN, 1.0)).err(),
        Some(ReadingValidationError::MeasurementValidationError(
            MeasurementValidationError::NotFinite {
                field: MeasurementField::Value,
            }
        )),
        "A not a number measurement must be rejected at the double precision"
    );

    // A field variant of the generic enum names its field and carries no parameter at all
    assert_eq!(
        Reading::<f64>::new(reading_draft(2.5, f64::NAN)).err(),
        Some(ReadingValidationError::NotFinite {
            field: ReadingField::Calibration,
        }),
        "A not a number calibration must be rejected at the double precision"
    );

    let double_error = Reading::<f64>::new(reading_draft(150.5, 1.0))
        .err()
        .expect("150.5 is above the ceiling");
    assert_eq!(
        double_error,
        ReadingValidationError::CeilingValidationError(AboveCeilingError { measurement: 150.5 }),
        "The final validation error must carry the rejected measurement"
    );
    assert_eq!(
        double_error.to_string(),
        "The measurement 150.5 is above the ceiling",
        "The generated enum must display the error that it holds"
    );
    let double_source = Error::source(&double_error).expect("A wrapper variant reports a source");
    assert_eq!(
        double_source.downcast_ref::<AboveCeilingError<f64>>(),
        Some(&AboveCeilingError { measurement: 150.5 }),
        "The source chain must reach the error of the final validation"
    );

    let new_measurement = Measurement::new(MeasurementDraft(3.5)).expect("3.5 is finite");
    assert_eq!(
        double.set_measurement(new_measurement),
        Ok(()),
        "A measurement below the ceiling must be accepted"
    );
    assert!(
        (*double.measurement().value() - 3.5_f64).abs() < f64::EPSILON,
        "The accepted patch must store the new measurement"
    );

    // The setter runs the final validation, which returns the generic error of the outer type
    let above_ceiling = Measurement::new(MeasurementDraft(150.5)).expect("150.5 is finite");
    assert_eq!(
        double.set_measurement(above_ceiling),
        Err(ReadingValidationError::CeilingValidationError(
            AboveCeilingError { measurement: 150.5 }
        )),
        "A patch that breaks the final validation must be rejected"
    );
    assert!(
        (*double.measurement().value() - 3.5_f64).abs() < f64::EPSILON,
        "A rejected patch must leave the measurement untouched"
    );

    let single = Reading::<f32>::new(reading_draft(1.5, 1.0)).expect("1.5 is below the ceiling");
    assert!(
        (*single.measurement().value() - 1.5_f32).abs() < f32::EPSILON,
        "The valid draft must build the reading at the single precision"
    );

    assert_eq!(
        Reading::<f32>::new(reading_draft(f32::INFINITY, 1.0)).err(),
        Some(ReadingValidationError::MeasurementValidationError(
            MeasurementValidationError::NotFinite {
                field: MeasurementField::Value,
            }
        )),
        "An infinite measurement must be rejected at the single precision"
    );

    let single_error = Reading::<f32>::new(reading_draft(120.5, 1.0))
        .err()
        .expect("120.5 is above the ceiling");
    assert_eq!(
        single_error,
        ReadingValidationError::CeilingValidationError(AboveCeilingError { measurement: 120.5 }),
        "The final validation error must carry the rejected measurement of its own precision"
    );
    assert_eq!(
        single_error.to_string(),
        "The measurement 120.5 is above the ceiling",
        "The generated enum must display the error of the single precision"
    );
    let single_source = Error::source(&single_error).expect("A wrapper variant reports a source");
    assert_eq!(
        single_source.downcast_ref::<AboveCeilingError<f32>>(),
        Some(&AboveCeilingError { measurement: 120.5 }),
        "The source chain must reach the error of the single precision"
    );

    let restored = Reading::<f32>::from_draft(single.to_draft()).expect("The draft stays valid");
    assert!(
        (*restored.measurement().value() - 1.5_f32).abs() < f32::EPSILON,
        "The draft round trip must keep the measurement"
    );
}
