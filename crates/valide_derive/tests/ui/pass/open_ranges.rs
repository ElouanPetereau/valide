//! Ranges that leave one end open, in the bound pair form and in the sugared form.

use core::ops::Bound;

/// Readings whose ranges leave one end open.
#[derive(valide_derive::Validate)]
struct Readings {
    /// Reading that accepts every value from zero upwards, written with a bound pair.
    #[validate(range(Bound::Included(0.0_f64), Bound::Unbounded))]
    bound_pair: f64,
    /// Reading that accepts every value from zero upwards, written with a sugared range.
    #[validate(range(0.0..))]
    open_upper: f64,
    /// Reading that accepts every value up to one, written with a sugared range.
    #[validate(range(..=1.0))]
    open_lower: f64,
}

/// Build a draft that every field validator accepts.
fn valid_draft() -> ReadingsDraft {
    ReadingsDraft {
        bound_pair: 0.0,
        open_upper: 0.0,
        open_lower: 1.0,
    }
}

fn main() {
    let mut infinite = valid_draft();
    infinite.bound_pair = f64::INFINITY;
    infinite.open_upper = f64::INFINITY;
    infinite.open_lower = f64::NEG_INFINITY;
    assert!(
        Readings::new(infinite).is_ok(),
        "An unbounded end must accept the infinity it reaches"
    );

    let mut below_the_pair = valid_draft();
    below_the_pair.bound_pair = -1.0;
    assert_eq!(
        Readings::new(below_the_pair).err(),
        Some(ReadingsValidationError::OutOfRange {
            field: ReadingsField::BoundPair,
            range: "[0.0, +inf]",
        }),
        "An unbounded upper end must be rendered as an included positive infinity"
    );

    let mut below_the_sugar = valid_draft();
    below_the_sugar.open_upper = -1.0;
    assert_eq!(
        Readings::new(below_the_sugar).err(),
        Some(ReadingsValidationError::OutOfRange {
            field: ReadingsField::OpenUpper,
            range: "[0.0, +inf]",
        }),
        "A missing upper bound must be rendered as an included positive infinity"
    );

    let mut above_the_sugar = valid_draft();
    above_the_sugar.open_lower = 2.0;
    assert_eq!(
        Readings::new(above_the_sugar).err(),
        Some(ReadingsValidationError::OutOfRange {
            field: ReadingsField::OpenLower,
            range: "[-inf, 1.0]",
        }),
        "A missing lower bound must be rendered as an included negative infinity"
    );
}
