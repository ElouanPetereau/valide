//! A named struct that carries every marker and a final validation.

use valide::{Patch as _, Validate as _};

/// Error of the sum check of a [`Whole`].
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
enum SumError {
    /// The parts weigh more than the whole.
    #[error("The parts must not weigh more than the whole")]
    PartsTooHeavy,
}

/// Fraction bounded to [0.0, 1.0].
#[derive(Clone, valide_derive::Validate, valide_derive::Patch)]
struct Fraction(
    /// The fraction itself.
    #[validate(range(0.0..=1.0))]
    f64,
);

/// A whole made of one part, with a nested fraction and a skipped label.
#[derive(Clone, valide_derive::Validate, valide_derive::Patch)]
#[final_validation(validate_sum, error = SumError)]
struct Whole {
    /// Mass of the whole.
    #[validate(range(0.0..f64::INFINITY))]
    whole: f64,
    /// Mass of the part.
    #[validate(finite)]
    part: f64,
    /// Fraction of the whole that the part covers.
    #[validate(nested)]
    fraction: Fraction,
    /// Label that takes part in no validation.
    #[validate(skip)]
    label: u8,
}

impl Whole {
    /// Check that the part of the given `draft` is not heavier than its whole.
    fn validate_sum(draft: &WholeDraft) -> Result<(), SumError> {
        if draft.whole < draft.part {
            return Err(SumError::PartsTooHeavy);
        }

        Ok(())
    }
}

/// Build a valid whole draft.
fn valid_draft() -> WholeDraft {
    WholeDraft {
        whole: 10.0,
        part: 4.0,
        fraction: FractionDraft(0.5),
        label: 7,
    }
}

fn main() {
    let mut whole = Whole::new(valid_draft()).expect("The draft is valid");
    assert_eq!(whole.label(), 7, "A skipped field must be passed through");

    let mut not_finite = valid_draft();
    not_finite.part = f64::NAN;
    assert_eq!(
        Whole::new(not_finite).err(),
        Some(WholeValidationError::NotFinite {
            field: WholeField::Part,
        }),
        "A finite field must reject a not a number value"
    );

    let mut too_heavy = valid_draft();
    too_heavy.part = 20.0;
    assert_eq!(
        Whole::new(too_heavy).err(),
        Some(WholeValidationError::SumValidationError(
            SumError::PartsTooHeavy
        )),
        "The final validation must run after the field validators"
    );

    let mut invalid_nested = valid_draft();
    invalid_nested.fraction = FractionDraft(2.0);
    assert_eq!(
        Whole::new(invalid_nested).err(),
        Some(WholeValidationError::FractionValidationError(
            FractionValidationError::OutOfRange {
                field: FractionField::Value,
                range: "[0.0, 1.0]",
            }
        )),
        "A nested field must wrap the error of its own type"
    );

    assert!(
        whole.set_part(50.0).is_err(),
        "A patch that breaks the final validation must be rejected"
    );
    assert!(
        (whole.part() - 4.0).abs() < f64::EPSILON,
        "A rejected patch must leave the value untouched"
    );

    // The type declares a final validation, so the setter of the skipped field runs it and returns
    // a result
    assert_eq!(
        whole.set_label(9),
        Ok(()),
        "A skipped field patch that keeps the final validation happy must be accepted"
    );
    assert_eq!(whole.label(), 9, "A skipped field setter must store the value");

    let restored = Whole::from_draft(whole.to_draft()).expect("The draft stays valid");
    assert!(
        (restored.whole() - 10.0).abs() < f64::EPSILON,
        "The draft round trip must keep the value"
    );
}
