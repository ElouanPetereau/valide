//! A type that carries two final validations, which run in declaration order.

/// Error of the order check of a [`Pair`].
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
enum OrderError {
    /// The lower value is above the upper value.
    #[error("The lower value must not be above the upper value")]
    LowerAboveUpper,
}

/// Error of the sum check of a [`Pair`].
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
enum SumError {
    /// The two values sum above ten.
    #[error("The two values must not sum above ten")]
    SumTooLarge,
}

/// An ordered pair of values whose sum stays below ten.
#[derive(valide_derive::Validate)]
#[final_validation(validate_order, error = OrderError)]
#[final_validation(validate_sum, error = SumError)]
struct Pair {
    /// Lower value of the pair.
    #[validate(finite)]
    lower: f64,
    /// Upper value of the pair.
    #[validate(finite)]
    upper: f64,
}

impl Pair {
    /// Check that the lower value of the given `draft` stays below its upper value.
    fn validate_order(draft: &PairDraft) -> Result<(), OrderError> {
        if draft.lower > draft.upper {
            return Err(OrderError::LowerAboveUpper);
        }

        Ok(())
    }

    /// Check that the two values of the given `draft` sum below ten.
    fn validate_sum(draft: &PairDraft) -> Result<(), SumError> {
        if draft.lower + draft.upper > 10.0 {
            return Err(SumError::SumTooLarge);
        }

        Ok(())
    }
}

fn main() {
    assert!(
        Pair::new(PairDraft {
            lower: 1.0,
            upper: 2.0
        })
        .is_ok(),
        "A draft that passes both final validations must be accepted"
    );

    // The draft breaks the two final validations at once, so the reported error names the one that
    // runs first
    assert_eq!(
        Pair::new(PairDraft {
            lower: 9.0,
            upper: 8.0
        })
        .err(),
        Some(PairValidationError::OrderValidationError(
            OrderError::LowerAboveUpper
        )),
        "The first declared final validation must run first"
    );

    assert_eq!(
        Pair::new(PairDraft {
            lower: 5.0,
            upper: 6.0
        })
        .err(),
        Some(PairValidationError::SumValidationError(
            SumError::SumTooLarge
        )),
        "The second final validation must wrap its error in its own variant"
    );
}
