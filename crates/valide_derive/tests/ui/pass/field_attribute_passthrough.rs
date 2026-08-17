//! Field level attributes that the generated draft must carry.
//!
//! The type carries a deserialization validation, so serde writes the wire format of the type and reads the wire format of its draft.
//! A renamed key only survives the round trip when the draft field carries the same rename.

use serde::{Deserialize, Serialize};

/// Payload of a spacecraft, whose mass field carries a renamed serde key.
#[derive(Serialize, Deserialize, valide_derive::Validate)]
#[serde(try_from = "PayloadDraft")]
struct Payload {
    /// Mass of the payload in kilograms (kg).
    #[serde(rename = "maxMass")]
    #[validate(range(0.0..=1000.0))]
    mass: f64,
    /// Fraction of the payload that the mission uses.
    // The field level passthrough gives the draft field a serde key that the type itself does not carry,
    // so a missing entry only defaults on the reading side
    #[draft_attr(serde(default))]
    #[validate(range(0.0..=1.0))]
    used_fraction: f64,
}

fn main() {
    let payload = Payload::new(PayloadDraft {
        mass: 12.5,
        used_fraction: 0.25,
    })
    .expect("The valid payload draft must build a payload");

    let document = serde_json::to_string(&payload).expect("A valid payload must serialize");
    assert!(
        document.contains("maxMass"),
        "The serialized document must carry the renamed key but was {document}"
    );

    let restored: Payload =
        serde_json::from_str(&document).expect("The serialized document must deserialize back");
    assert_eq!(
        restored.mass().to_bits(),
        12.5_f64.to_bits(),
        "The renamed mass must survive the round trip"
    );
    assert_eq!(
        restored.used_fraction().to_bits(),
        0.25_f64.to_bits(),
        "The used fraction must survive the round trip"
    );

    let defaulted: Payload = serde_json::from_str("{\"maxMass\":3.0}")
        .expect("A document without the used fraction must deserialize");
    assert_eq!(
        defaulted.used_fraction().to_bits(),
        0.0_f64.to_bits(),
        "The field level passthrough must give the draft field its serde default"
    );
}
