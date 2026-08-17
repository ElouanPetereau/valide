//! An integer range, whose bounds carry no floating point literal.

/// A count bounded to [0, 10].
#[derive(valide_derive::Validate)]
struct Count {
    /// The count itself.
    #[validate(range(0..=10))]
    value: u8,
}

fn main() {
    let count = Count::new(CountDraft { value: 10 }).expect("10 is inside the range");
    assert_eq!(
        count.value(),
        10,
        "The upper bound of an inclusive integer range must be accepted"
    );

    assert_eq!(
        Count::new(CountDraft { value: 11 }).err(),
        Some(CountValidationError::OutOfRange {
            field: CountField::Value,
            range: "[0, 10]",
        }),
        "A count above the range must be rejected with the rendered integer range"
    );
}
