//! Fields and a variant payload written as an `Option`.
//!
//! A `None` value passes every generated check and a `Some` value is checked through its payload.
//! A custom function receives the whole `Option` and decides what `None` means.

use core::{fmt::Debug, ops::Bound};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use valide::{Patch as _, Validate as _};

/// Standard gravitational parameter of the Earth in cubic meters per second squared (m³/s²).
const EARTH_GRAVITATIONAL_PARAMETER: f64 = 3.986_004_418e14;

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

/// Fraction bounded to [0.0, 1.0].
#[derive(
    Clone, PartialEq, Debug, Serialize, Deserialize, valide_derive::Validate, valide_derive::Patch,
)]
#[serde(try_from = "FractionDraft")]
#[draft_attr(derive(Debug, PartialEq))]
struct Fraction(
    /// The fraction itself.
    #[validate(range(0.0..=1.0))]
    f64,
);

/// Celestial body with a strictly positive gravitational parameter.
#[derive(
    Clone, PartialEq, Debug, Serialize, Deserialize, valide_derive::Validate, valide_derive::Patch,
)]
#[serde(try_from = "BodyDraft")]
struct Body {
    /// Standard gravitational parameter in cubic meters per second squared (m³/s²).
    #[validate(range(Bound::Excluded(0.0), Bound::Excluded(f64::INFINITY)))]
    gravitational_parameter: f64,
}

/// Target of a probe.
#[derive(
    Clone, PartialEq, Debug, Serialize, Deserialize, valide_derive::Validate, valide_derive::Patch,
)]
#[serde(try_from = "TargetDraft")]
enum Target {
    /// The probe holds its position.
    Idle,
    /// The probe orbits a body, which stays unknown until the approach.
    Orbit(#[validate(nested)] Option<Body>),
}

/// Error of the callsign check of a [`Probe`].
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
enum CallsignError {
    /// The callsign carries no character.
    #[error("The callsign must carry at least one character")]
    Empty,
}

/// Probe whose every field may hold nothing.
#[derive(Clone, Serialize, Deserialize, valide_derive::Validate, valide_derive::Patch)]
#[serde(try_from = "ProbeDraft")]
struct Probe {
    /// Propellant mass in kilograms (kg), absent before the fueling.
    #[validate(range(0.0..=5_000.0))]
    propellant_mass: Option<f64>,
    /// Mass of the solar sail in kilograms (kg), absent on a probe without a sail.
    #[validate(range(Bound::Excluded(0.0), Bound::Included(10_000.0)))]
    sail_mass: Option<f64>,
    /// Altitude of the perigee in kilometers (km), absent before the orbit insertion.
    #[validate(range(100.0..))]
    perigee_altitude: Option<f64>,
    /// Drag coefficient, absent before the first estimation.
    #[validate(finite)]
    drag_coefficient: Option<f64>,
    /// Fraction of the orbit spent in the shadow of the body, unknown before the first orbit.
    #[validate(nested)]
    eclipse_shadow_fraction: Option<Fraction>,
    /// Target of the probe, absent while the mission plans none.
    #[validate(nested)]
    destination: Option<Target>,
    /// Callsign of the probe, absent before the registration.
    #[validate(custom(check_callsign, error = CallsignError))]
    callsign: Option<String>,
    /// Free comment, which takes part in no validation.
    #[validate(skip)]
    comment: Option<String>,
}

impl Probe {
    /// Check that the given `callsign` carries at least one character when it holds one.
    /// A missing callsign passes.
    fn check_callsign(callsign: &Option<String>) -> Result<(), CallsignError> {
        if callsign.as_ref().is_some_and(String::is_empty) {
            return Err(CallsignError::Empty);
        }

        Ok(())
    }
}

/// Fraction bounded to [0.0, 1.0] that may hold nothing.
#[derive(valide_derive::Validate)]
struct MaybeFraction(
    /// The fraction itself, absent while unknown.
    #[validate(range(0.0..=1.0))]
    Option<f64>,
);

/// Solar panel whose two measurements may hold nothing, generic over the precision.
/// The two fields reach the generated error enum, which needs the `Debug` of the parameter.
#[derive(valide_derive::Validate)]
struct Panel<Number: Precision + Debug> {
    /// Area in square meters (m²), absent before the deployment.
    #[validate(range(..=Number::CEILING))]
    area: Option<Number>,
    /// Tilt in radians, absent before the deployment.
    #[validate(finite)]
    tilt: Option<Number>,
}

/// Build a probe draft whose every field holds nothing.
fn none_probe_draft() -> ProbeDraft {
    ProbeDraft {
        propellant_mass: None,
        sail_mass: None,
        perigee_altitude: None,
        drag_coefficient: None,
        eclipse_shadow_fraction: None,
        destination: None,
        callsign: None,
        comment: None,
    }
}

/// Build a probe draft whose every field holds a valid value.
fn some_probe_draft() -> ProbeDraft {
    ProbeDraft {
        propellant_mass: Some(2_000.0),
        sail_mass: Some(10.0),
        perigee_altitude: Some(400.0),
        drag_coefficient: Some(2.2),
        eclipse_shadow_fraction: Some(FractionDraft(0.5)),
        destination: Some(TargetDraft::Orbit(Some(BodyDraft {
            gravitational_parameter: EARTH_GRAVITATIONAL_PARAMETER,
        }))),
        callsign: Some("PROBE-1".to_owned()),
        comment: Some("Launched".to_owned()),
    }
}

fn main() {
    // A probe whose every field holds nothing
    let empty_probe = Probe::new(none_probe_draft()).expect("A draft that holds nothing is valid");
    assert_eq!(
        empty_probe.eclipse_shadow_fraction(),
        &None,
        "A missing nested field must stay missing"
    );

    // A probe whose every field holds a valid value
    let mut probe = Probe::new(some_probe_draft()).expect("Every field holds a valid value");
    let half = Fraction::new(FractionDraft(0.5)).expect("0.5 is inside the range of a fraction");
    let earth = Body::new(BodyDraft {
        gravitational_parameter: EARTH_GRAVITATIONAL_PARAMETER,
    })
    .expect("The gravitational parameter of the Earth is positive");
    assert_eq!(
        (
            probe.propellant_mass(),
            probe.sail_mass(),
            probe.perigee_altitude(),
        ),
        (&Some(2_000.0), &Some(10.0), &Some(400.0)),
        "The getter of a range field must return the value"
    );
    assert_eq!(
        probe.drag_coefficient(),
        &Some(2.2),
        "The getter of a finite field must return the value"
    );
    assert_eq!(
        probe.eclipse_shadow_fraction(),
        &Some(half.clone()),
        "The getter of a nested field must return the value"
    );
    assert_eq!(
        probe.destination(),
        &Some(Target::Orbit(Some(earth))),
        "The getter of a nested enum field must return the variant and its payload"
    );
    assert_eq!(
        probe.callsign(),
        &Some("PROBE-1".to_owned()),
        "The getter of a custom field must return the value"
    );
    assert_eq!(
        probe.comment(),
        &Some("Launched".to_owned()),
        "The getter of a skip field must return the value"
    );

    // A range field whose payload is above the upper bound
    let above_capacity = Probe::new(ProbeDraft {
        propellant_mass: Some(6_000.0),
        ..none_probe_draft()
    })
    .err()
    .expect("6000.0 is above the capacity of the tank");
    assert_eq!(
        above_capacity,
        ProbeValidationError::PropellantMassOutOfRange {
            lower: Bound::Included(0.0),
            upper: Bound::Included(5_000.0),
            value: 6_000.0,
        },
        "A range field must check the payload of its Some value"
    );
    assert_eq!(
        above_capacity.to_string(),
        "propellant_mass must be within the range [0.0, 5000.0]",
        "The rejection of a range field must display the bounds of its payload type"
    );

    // A range field in the bound pair form whose payload sits on the excluded lower bound
    assert_eq!(
        Probe::new(ProbeDraft {
            sail_mass: Some(0.0),
            ..none_probe_draft()
        })
        .err(),
        Some(ProbeValidationError::SailMassOutOfRange {
            lower: Bound::Excluded(0.0),
            upper: Bound::Included(10_000.0),
            value: 0.0,
        }),
        "A range field in the bound pair form must check the payload of its Some value"
    );

    // A range field without an upper end whose payload is below the lower bound
    assert_eq!(
        Probe::new(ProbeDraft {
            perigee_altitude: Some(50.0),
            ..none_probe_draft()
        })
        .err(),
        Some(ProbeValidationError::PerigeeAltitudeOutOfRange {
            lower: Bound::Included(100.0),
            upper: Bound::Unbounded,
            value: 50.0,
        }),
        "A range field without an upper end must check the payload of its Some value"
    );

    // A finite field whose payload is not a number.
    // A not a number value equals no value at all, so the rejection cannot be compared
    let rejected_drag = Probe::new(ProbeDraft {
        drag_coefficient: Some(f64::NAN),
        ..none_probe_draft()
    })
    .err()
    .expect("A not a number drag coefficient must be rejected");
    let ProbeValidationError::DragCoefficientNotFinite { value } = rejected_drag else {
        panic!("The rejection must be the finite variant of the drag coefficient field");
    };
    assert!(
        value.is_nan(),
        "A finite field must check the payload of its Some value and carry the rejected payload"
    );

    // A nested field whose payload breaks its own range
    assert_eq!(
        Probe::new(ProbeDraft {
            eclipse_shadow_fraction: Some(FractionDraft(1.5)),
            ..none_probe_draft()
        })
        .err(),
        Some(ProbeValidationError::EclipseShadowFractionValidationError(
            FractionValidationError::ValueOutOfRange {
                lower: Bound::Included(0.0),
                upper: Bound::Included(1.0),
                value: 1.5,
            }
        )),
        "A nested field must wrap the error of its payload"
    );

    // A nested enum field whose optional payload breaks its own range
    assert_eq!(
        Probe::new(ProbeDraft {
            destination: Some(TargetDraft::Orbit(Some(BodyDraft {
                gravitational_parameter: 0.0,
            }))),
            ..none_probe_draft()
        })
        .err(),
        Some(ProbeValidationError::DestinationValidationError(
            TargetValidationError::OrbitValidationError(
                BodyValidationError::GravitationalParameterOutOfRange {
                    lower: Bound::Excluded(0.0),
                    upper: Bound::Excluded(f64::INFINITY),
                    value: 0.0,
                }
            )
        )),
        "A nested enum field must wrap the error of its variant payload"
    );

    // A custom field that its function rejects
    assert_eq!(
        Probe::new(ProbeDraft {
            callsign: Some(String::new()),
            ..none_probe_draft()
        })
        .err(),
        Some(ProbeValidationError::CallsignValidationError(
            CallsignError::Empty
        )),
        "A custom field must wrap the error that its function returns"
    );

    // The setter of a range field checks the payload of its Some value
    assert_eq!(
        probe.set_propellant_mass(Some(6_000.0)),
        Err(ProbeValidationError::PropellantMassOutOfRange {
            lower: Bound::Included(0.0),
            upper: Bound::Included(5_000.0),
            value: 6_000.0,
        }),
        "The setter of a range field must reject a payload outside the range"
    );
    assert_eq!(
        probe.propellant_mass(),
        &Some(2_000.0),
        "A rejected patch must leave the value untouched"
    );

    // The setters accept a value and accept nothing
    assert_eq!(
        probe.set_propellant_mass(None),
        Ok(()),
        "The setter of a range field must accept nothing"
    );
    assert_eq!(
        probe.propellant_mass(),
        &None,
        "The accepted patch must store nothing"
    );
    assert_eq!(
        probe.set_propellant_mass(Some(2.0)),
        Ok(()),
        "The setter of a range field must accept a payload inside the range"
    );
    assert_eq!(
        probe.propellant_mass(),
        &Some(2.0),
        "The accepted patch must store the new value"
    );
    let quarter =
        Fraction::new(FractionDraft(0.25)).expect("0.25 is inside the range of a fraction");
    assert_eq!(
        probe.set_eclipse_shadow_fraction(Some(quarter.clone())),
        Ok(()),
        "The setter of a nested field must accept a valid value"
    );
    assert_eq!(
        probe.eclipse_shadow_fraction(),
        &Some(quarter),
        "The accepted patch must store the new value"
    );
    assert_eq!(
        probe.set_destination(None),
        Ok(()),
        "The setter of a nested field must accept nothing"
    );
    assert_eq!(
        probe.destination(),
        &None,
        "The accepted patch must store nothing"
    );

    // The draft round trip of an optional nested field
    let original = Probe::new(some_probe_draft()).expect("Every field holds a valid value");
    assert_eq!(
        original.to_draft().eclipse_shadow_fraction,
        Some(FractionDraft(0.5)),
        "The draft of an optional nested field must hold the draft of its payload"
    );
    let restored = Probe::from_draft(original.to_draft()).expect("The draft stays valid");
    assert_eq!(
        restored.eclipse_shadow_fraction(),
        &Some(half),
        "The draft round trip must keep the value of an optional nested field"
    );

    // A document without a key gives a probe whose every field holds nothing
    let deserialized =
        serde_json::from_str::<Probe>("{}").expect("A document without a key is valid");
    assert_eq!(
        (
            deserialized.propellant_mass(),
            deserialized.sail_mass(),
            deserialized.perigee_altitude(),
            deserialized.drag_coefficient(),
        ),
        (&None, &None, &None, &None),
        "A missing key must give nothing in a range field and in a finite field"
    );
    assert_eq!(
        deserialized.eclipse_shadow_fraction(),
        &None,
        "A missing key must give nothing in a nested field"
    );
    assert_eq!(
        deserialized.destination(),
        &None,
        "A missing key must give nothing in a nested enum field"
    );
    assert_eq!(
        deserialized.callsign(),
        &None,
        "A missing key must give nothing in a custom field"
    );
    assert_eq!(
        deserialized.comment(),
        &None,
        "A missing key must give nothing in a skip field"
    );

    // A document whose payload breaks its range, on a range field and on a nested field
    for rejected_document in [
        r#"{"propellant_mass": 6000.0}"#,
        r#"{"eclipse_shadow_fraction": 1.5}"#,
    ] {
        let message = serde_json::from_str::<Probe>(rejected_document)
            .err()
            .expect("The document holds a payload outside its range")
            .to_string();
        assert!(
            message.contains("range"),
            "The rejection of {rejected_document} must mention the range but was {message}"
        );
    }

    // A field that holds nothing serializes to null
    let document = serde_json::to_value(&empty_probe).expect("A probe serializes to a document");
    for key in [
        "propellant_mass",
        "sail_mass",
        "perigee_altitude",
        "drag_coefficient",
        "eclipse_shadow_fraction",
        "destination",
        "callsign",
        "comment",
    ] {
        assert_eq!(
            document.get(key),
            Some(&Value::Null),
            "The {key} key must hold null while its field holds nothing"
        );
    }

    // An enum whose variant payload is optional
    assert_eq!(
        Target::new(TargetDraft::Orbit(None)).expect("A payload that holds nothing is valid"),
        Target::Orbit(None),
        "A variant payload that holds nothing must pass"
    );
    assert_eq!(
        Target::new(TargetDraft::Orbit(Some(BodyDraft {
            gravitational_parameter: 0.0,
        })))
        .err(),
        Some(TargetValidationError::OrbitValidationError(
            BodyValidationError::GravitationalParameterOutOfRange {
                lower: Bound::Excluded(0.0),
                upper: Bound::Excluded(f64::INFINITY),
                value: 0.0,
            }
        )),
        "A variant payload must wrap the error of its own payload type"
    );

    // A newtype whose single field may hold nothing
    let unknown =
        MaybeFraction::new(MaybeFractionDraft(None)).expect("Nothing is a valid fraction");
    assert_eq!(
        unknown.value(),
        &None,
        "A newtype that holds nothing must pass"
    );
    let known = MaybeFraction::new(MaybeFractionDraft(Some(0.5)))
        .expect("0.5 is inside the range of a fraction");
    assert_eq!(
        known.value(),
        &Some(0.5),
        "A newtype whose payload is inside the range must pass"
    );
    assert_eq!(
        MaybeFraction::new(MaybeFractionDraft(Some(1.5))).err(),
        Some(MaybeFractionValidationError::ValueOutOfRange {
            lower: Bound::Included(0.0),
            upper: Bound::Included(1.0),
            value: 1.5,
        }),
        "A newtype must check the payload of its Some value"
    );

    // A generic struct whose optional fields read the bounds and the check of their parameter
    assert_eq!(
        Panel::<f32>::new(PanelDraft {
            area: Some(f32::INFINITY),
            tilt: None,
        })
        .err(),
        Some(PanelValidationError::AreaOutOfRange {
            lower: Bound::Unbounded,
            upper: Bound::Included(100.0),
            value: f32::INFINITY,
        }),
        "A generic range field must check its payload with the bounds of the single precision"
    );
    // A not a number value equals no value at all, so the rejection cannot be compared
    let rejected_tilt = Panel::<f32>::new(PanelDraft {
        area: None,
        tilt: Some(f32::NAN),
    })
    .err()
    .expect("A not a number tilt must be rejected");
    let PanelValidationError::TiltNotFinite { value } = rejected_tilt else {
        panic!("The rejection must be the finite variant of the tilt field");
    };
    assert!(
        value.is_nan(),
        "A generic finite field must check its payload and carry the rejected payload"
    );
    let stowed = Panel::<f32>::new(PanelDraft {
        area: None,
        tilt: None,
    })
    .expect("A panel that holds nothing is valid");
    assert_eq!(
        (stowed.area(), stowed.tilt()),
        (&None, &None),
        "A generic struct whose optional fields hold nothing must pass"
    );
}
