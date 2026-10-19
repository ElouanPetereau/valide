//! An enum with struct variants, whose nested fields carry every rule of the enum.
//!
//! A struct variant without a field stays a struct variant, each nested field reaches its own wrapper variant
//! and a skip field takes part in no validation.

use core::{
    error::Error,
    fmt::{Debug, Display},
    ops::Bound,
};

use valide::{Patch as _, Validate as _};

/// Standard gravitational parameter of the Earth in cubic meters per second squared (m³/s²).
const EARTH_GRAVITATIONAL_PARAMETER: f64 = 3.986_004_418e14;

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

/// Fraction bounded to [0.0, 1.0].
#[derive(Clone, PartialEq, Debug, valide_derive::Validate, valide_derive::Patch)]
struct Fraction(
    /// The fraction itself.
    #[validate(range(0.0..=1.0))]
    f64,
);

/// Celestial body with a strictly positive gravitational parameter.
#[derive(Clone, PartialEq, Debug, valide_derive::Validate, valide_derive::Patch)]
struct CelestialBody {
    /// Standard gravitational parameter in cubic meters per second squared (m³/s²).
    #[validate(range(Bound::Excluded(0.0), Bound::Excluded(f64::INFINITY)))]
    gravitational_parameter: f64,
}

/// Measurement that must be a finite number at its own precision.
/// The finite field reaches the generated error enum, which needs the `Debug` of the parameter.
#[derive(Clone, PartialEq, Debug, valide_derive::Validate, valide_derive::Patch)]
struct Measurement<Number: Precision + Debug>(
    /// The measured value itself.
    #[validate(finite)]
    Number,
);

/// Pointing of a spacecraft.
#[derive(Clone, PartialEq, Debug, valide_derive::Validate, valide_derive::Patch)]
enum Pointing {
    /// Point at the Sun, a struct variant without a field.
    SunPointing {},
    /// Point at the ground, a unit variant.
    Nadir,
    /// Point at a custom body, with the fraction of the pass to track.
    Custom {
        /// Body to point at.
        #[validate(nested)]
        body: CelestialBody,
        /// Fraction of the pass to track.
        #[validate(nested)]
        fraction: Fraction,
        /// Free comment, which takes part in no validation.
        #[validate(skip)]
        comment: String,
    },
    /// Orbit a body that stays unknown until the approach.
    Orbit {
        /// Body to orbit, absent while unknown.
        #[validate(nested)]
        body: Option<CelestialBody>,
    },
    /// Raw pointing command, with a raw identifier field.
    Raw {
        /// Kind of the command.
        #[validate(skip)]
        r#type: u8,
        /// Label of the command.
        #[validate(skip)]
        label: String,
    },
}

/// Sample that a sensor reported at its own precision on one of its channels.
#[derive(Clone, PartialEq, Debug, valide_derive::Validate, valide_derive::Patch)]
enum Sample<Number>
where
    Number: Precision + Debug + Display + 'static,
{
    /// The sensor reported no value at all.
    Missing,
    /// Measurement that the sensor reported on one of its channels.
    Measured {
        /// Measurement that the sensor reported.
        #[validate(nested)]
        measurement: Measurement<Number>,
        /// Channel of the sensor, which takes part in no validation.
        #[validate(skip)]
        channel: u8,
    },
}

/// Return the draft of the Earth, which is a valid celestial body.
fn earth_draft() -> CelestialBodyDraft {
    CelestialBodyDraft {
        gravitational_parameter: EARTH_GRAVITATIONAL_PARAMETER,
    }
}

/// Return the draft of a valid custom pointing, which tracks a quarter of the pass over the Earth.
fn custom_draft() -> PointingDraft {
    PointingDraft::Custom {
        body: earth_draft(),
        fraction: FractionDraft(0.25),
        comment: "Track the Earth".to_owned(),
    }
}

fn main() {
    assert_eq!(
        Pointing::new(PointingDraft::SunPointing {})
            .expect("A struct variant without a field is always valid"),
        Pointing::SunPointing {},
        "A struct variant without a field must build the matching variant"
    );

    assert_eq!(
        Pointing::new(PointingDraft::Nadir).expect("A unit variant is always valid"),
        Pointing::Nadir,
        "A unit variant must build the matching variant"
    );

    let custom = Pointing::new(custom_draft()).expect("The custom draft is valid");
    assert_eq!(
        custom,
        Pointing::Custom {
            body: CelestialBody::new(earth_draft()).expect("The Earth is valid"),
            fraction: Fraction::new(FractionDraft(0.25)).expect("0.25 is inside the range"),
            comment: "Track the Earth".to_owned(),
        },
        "Valid nested fields must build the variant and the skip field must pass through"
    );
    assert_eq!(
        Pointing::new(PointingDraft::Raw {
            r#type: 3,
            label: "Slew".to_owned(),
        })
        .expect("Skip fields are always valid"),
        Pointing::Raw {
            r#type: 3,
            label: "Slew".to_owned(),
        },
        "A raw identifier field must pass through every generated item"
    );

    let body_rejection = Pointing::new(PointingDraft::Custom {
        body: CelestialBodyDraft {
            gravitational_parameter: 0.0,
        },
        fraction: FractionDraft(0.25),
        comment: "Track nothing".to_owned(),
    })
    .err()
    .expect("A zero gravitational parameter is outside the range");
    assert_eq!(
        body_rejection,
        PointingValidationError::CustomBodyValidationError(
            CelestialBodyValidationError::GravitationalParameterOutOfRange {
                lower: Bound::Excluded(0.0),
                upper: Bound::Excluded(f64::INFINITY),
                value: 0.0,
            }
        ),
        "A nested field must wrap the error of its own type"
    );

    let fraction_rejection = Pointing::new(PointingDraft::Custom {
        body: earth_draft(),
        fraction: FractionDraft(1.5),
        comment: "Track too much".to_owned(),
    })
    .err()
    .expect("1.5 is outside the range of a fraction");
    assert_eq!(
        fraction_rejection,
        PointingValidationError::CustomFractionValidationError(
            FractionValidationError::ValueOutOfRange {
                lower: Bound::Included(0.0),
                upper: Bound::Included(1.0),
                value: 1.5,
            }
        ),
        "Each nested field must reach its own wrapper variant"
    );

    assert_eq!(
        Pointing::new(PointingDraft::Custom {
            body: CelestialBodyDraft {
                gravitational_parameter: 0.0,
            },
            fraction: FractionDraft(1.5),
            comment: "Track nothing at all".to_owned(),
        })
        .err(),
        Some(body_rejection),
        "The validation must stop at the first invalid field in declaration order"
    );

    assert_eq!(
        fraction_rejection.to_string(),
        "value must be within the range [0.0, 1.0]",
        "The wrapper variant must display the error that it holds"
    );
    // The wrapper variant already displays the error of the field, so it forwards the source of
    // that error instead of reporting the error itself
    assert!(
        Error::source(&fraction_rejection).is_none(),
        "The wrapper variant must forward the source of the field error, which reports none"
    );

    assert_eq!(
        Pointing::new(PointingDraft::Orbit { body: None }).expect("An absent body passes"),
        Pointing::Orbit { body: None },
        "An absent nested field must pass the validation"
    );
    assert_eq!(
        Pointing::new(PointingDraft::Orbit {
            body: Some(CelestialBodyDraft {
                gravitational_parameter: 0.0,
            }),
        })
        .err(),
        Some(PointingValidationError::OrbitBodyValidationError(
            CelestialBodyValidationError::GravitationalParameterOutOfRange {
                lower: Bound::Excluded(0.0),
                upper: Bound::Excluded(f64::INFINITY),
                value: 0.0,
            }
        )),
        "A present nested field must be checked through its payload"
    );

    let restored = Pointing::from_draft(custom.to_draft()).expect("The draft stays valid");
    assert_eq!(
        restored, custom,
        "The draft round trip must keep the variant and its fields"
    );
    assert_eq!(
        Pointing::from_draft(Pointing::SunPointing {}.to_draft())
            .expect("A struct variant without a field is always valid"),
        Pointing::SunPointing {},
        "The draft round trip must keep a struct variant without a field"
    );

    assert_eq!(
        Sample::<f32>::new(SampleDraft::Measured {
            measurement: MeasurementDraft(f32::INFINITY),
            channel: 1,
        })
        .err(),
        Some(SampleValidationError::MeasuredMeasurementValidationError(
            MeasurementValidationError::ValueNotFinite {
                value: f32::INFINITY,
            }
        )),
        "An infinite measurement must be rejected at the single precision"
    );
    let double = Sample::<f64>::new(SampleDraft::Measured {
        measurement: MeasurementDraft(2.5),
        channel: 1,
    })
    .expect("2.5 is a finite measurement");
    assert_eq!(
        double,
        Sample::Measured {
            measurement: Measurement::new(MeasurementDraft(2.5_f64))
                .expect("2.5 is a finite measurement"),
            channel: 1,
        },
        "A valid nested field must build the sample at the double precision"
    );
    assert_eq!(
        Sample::<f64>::from_draft(double.to_draft()).expect("The draft stays valid"),
        double,
        "The draft round trip must keep the variant of the double precision sample"
    );
}
