//! Fields and a variant payload written as an `Option`.
//!
//! A `None` value passes every generated check and a `Some` value is checked through its payload.
//! A custom function receives the whole `Option` and decides what `None` means.

use core::ops::Bound;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use valide::{Patch as _, Validate as _};

/// Standard gravitational parameter of the Earth in cubic meters per second squared (m³/s²).
const EARTH_GRAVITATIONAL_PARAMETER: f64 = 3.986_004_418e14;

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

/// Build a probe draft whose every field holds nothing.
fn none_probe_draft() -> ProbeDraft {
    ProbeDraft {
        eclipse_shadow_fraction: None,
        destination: None,
        callsign: None,
        comment: None,
    }
}

/// Build a probe draft whose every field holds a valid value.
fn some_probe_draft() -> ProbeDraft {
    ProbeDraft {
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

    // The setters of the nested fields accept a value and accept nothing
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

    // A document whose nested payload breaks its own range
    let message = serde_json::from_str::<Probe>(r#"{"eclipse_shadow_fraction": 1.5}"#)
        .err()
        .expect("1.5 is outside the range of a fraction")
        .to_string();
    assert!(
        message.contains("range"),
        "The rejection must mention the range but was {message}"
    );

    // A field that holds nothing serializes to null
    let document = serde_json::to_value(&empty_probe).expect("A probe serializes to a document");
    for key in [
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
}
