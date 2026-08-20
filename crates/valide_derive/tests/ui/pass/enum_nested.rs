//! An enum with a unit variant, a skipped payload and a nested payload.
//!
//! The nested payload carries every rule of the enum, so a public variant constructor bypasses nothing.

use core::error::Error;

use serde::{Deserialize, Serialize};
use valide::{Patch as _, Validate as _};

/// Fraction bounded to [0.0, 1.0].
#[derive(
    Clone, PartialEq, Debug, Serialize, Deserialize, valide_derive::Validate, valide_derive::Patch,
)]
#[serde(try_from = "FractionDraft")]
struct Fraction(
    /// The fraction itself.
    #[validate(range(0.0..=1.0))]
    f64,
);

/// Command of an actuator.
#[derive(
    Clone, PartialEq, Debug, Serialize, Deserialize, valide_derive::Validate, valide_derive::Patch,
)]
#[serde(try_from = "CommandDraft")]
enum Command {
    /// Stop the actuator.
    Halt,
    /// Raw code that takes part in no validation.
    Raw(#[validate(skip)] u8),
    /// Fraction of the stroke that the actuator must reach.
    Extend(#[validate(nested)] Fraction),
}

fn main() {
    assert_eq!(
        Command::new(CommandDraft::Halt).expect("A unit variant is always valid"),
        Command::Halt,
        "A unit variant must build the matching variant"
    );

    assert_eq!(
        Command::new(CommandDraft::Raw(7)).expect("A skipped payload is always valid"),
        Command::Raw(7),
        "A skipped payload must be passed through"
    );

    let extended = Command::new(CommandDraft::Extend(FractionDraft(0.25)))
        .expect("0.25 is inside the range of a fraction");
    assert_eq!(
        extended,
        Command::Extend(Fraction::new(FractionDraft(0.25)).expect("0.25 is inside the range")),
        "A valid nested payload must build the matching variant"
    );

    let rejection = Command::new(CommandDraft::Extend(FractionDraft(1.5)))
        .err()
        .expect("1.5 is outside the range of a fraction");
    assert_eq!(
        rejection,
        CommandValidationError::ExtendValidationError(FractionValidationError::OutOfRange {
            field: FractionField::Value,
            range: "[0.0, 1.0]",
        }),
        "A nested variant must wrap the error of its own payload type"
    );
    assert_eq!(
        rejection.to_string(),
        "The value must be within the range [0.0, 1.0]",
        "The wrapper variant must display the error that it holds"
    );
    let source = Error::source(&rejection).expect("A wrapper variant reports a source");
    assert_eq!(
        source.downcast_ref::<FractionValidationError>(),
        Some(&FractionValidationError::OutOfRange {
            field: FractionField::Value,
            range: "[0.0, 1.0]",
        }),
        "The source chain must reach the error of the payload type"
    );

    let deserialized: Command = serde_json::from_str(r#"{"Extend":0.5}"#)
        .expect("0.5 is inside the range of a fraction");
    assert_eq!(
        deserialized,
        Command::Extend(Fraction::new(FractionDraft(0.5)).expect("0.5 is inside the range")),
        "The validation must accept a valid document"
    );
    assert!(
        serde_json::from_str::<Command>(r#"{"Extend":1.5}"#).is_err(),
        "The validation must reject an invalid document"
    );
    assert_eq!(
        serde_json::to_string(&Command::Halt).expect("A unit variant serializes to its own name"),
        r#""Halt""#,
        "The wire format of a unit variant must stay its own name"
    );

    let restored = Command::from_draft(extended.to_draft()).expect("The draft stays valid");
    assert_eq!(
        restored, extended,
        "The draft round trip must keep the variant and its payload"
    );
}
