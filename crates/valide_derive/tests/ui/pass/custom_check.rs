//! A named struct whose field carries a custom validation function.

use core::error::Error as _;

use valide::{Patch as _, Validate as _};

/// Error of the designation check of a [`Reading`].
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
enum DesignationError {
    /// The designation carries no character.
    #[error("The designation must carry at least one character")]
    Empty,
}

/// Designation of a reading, which the custom check guards.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Designation(String);

/// A reading with a bounded fraction and a custom checked designation.
// The `Debug` implementation lets the assertions read the rejection of a construction
#[derive(Clone, Debug, valide_derive::Validate, valide_derive::Patch)]
struct Reading {
    /// Fraction of the reading.
    #[validate(range(0.0..=1.0))]
    fraction: f64,
    /// Designation of the reading.
    #[validate(custom(check_designation, error = DesignationError))]
    designation: Designation,
}

impl Reading {
    /// Check that the given `designation` carries at least one character.
    fn check_designation(designation: &Designation) -> Result<(), DesignationError> {
        if designation.0.is_empty() {
            return Err(DesignationError::Empty);
        }

        Ok(())
    }
}

/// Build a valid reading draft.
fn valid_draft() -> ReadingDraft {
    ReadingDraft {
        fraction: 0.5,
        designation: Designation("IMAGER".to_owned()),
    }
}

fn main() {
    let mut reading = Reading::new(valid_draft()).expect("The draft is valid");
    assert_eq!(
        reading.designation(),
        &Designation("IMAGER".to_owned()),
        "A custom field must keep its declared type and its value"
    );

    let mut empty_designation = valid_draft();
    empty_designation.designation = Designation(String::new());
    let rejection = Reading::new(empty_designation).expect_err("The designation is empty");
    assert_eq!(
        rejection,
        ReadingValidationError::DesignationValidationError(DesignationError::Empty),
        "A custom field must wrap the error value that its function returns"
    );
    assert_eq!(
        rejection.to_string(),
        "The designation must carry at least one character",
        "The wrapper must display the error that it holds"
    );
    assert_eq!(
        rejection.source().map(ToString::to_string),
        Some("The designation must carry at least one character".to_owned()),
        "The wrapper must report the error that it holds as its source"
    );

    assert_eq!(
        reading.set_designation(Designation("RADAR".to_owned())),
        Ok(()),
        "A patch that passes the custom check must be accepted"
    );
    assert!(
        reading.set_designation(Designation(String::new())).is_err(),
        "A patch that fails the custom check must be rejected"
    );
    assert_eq!(
        reading.designation(),
        &Designation("RADAR".to_owned()),
        "A rejected patch must leave the value untouched"
    );

    // The fraction comes first in the declaration order, so its error is the only one that a draft
    // breaking the two rules can report
    let mut both_invalid = valid_draft();
    both_invalid.fraction = 2.0;
    both_invalid.designation = Designation(String::new());
    assert_eq!(
        Reading::new(both_invalid).err(),
        Some(ReadingValidationError::OutOfRange {
            field: ReadingField::Fraction,
            range: "[0.0, 1.0]",
        }),
        "The fail fast policy must stop before the custom check of a later field"
    );

    let restored = Reading::from_draft(reading.to_draft()).expect("The draft stays valid");
    assert_eq!(
        restored.designation(),
        &Designation("RADAR".to_owned()),
        "The draft round trip must keep the value of a custom field"
    );
}
