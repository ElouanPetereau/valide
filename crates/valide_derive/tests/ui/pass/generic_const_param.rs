//! A validated type with a const parameter, which its range and its skipped field both read.

/// Window of a fixed sample count.
#[derive(valide_derive::Validate)]
struct Window<const LENGTH: usize> {
    /// Index of the current sample inside the window.
    #[validate(range(0..=LENGTH))]
    index: usize,
    /// Samples of the window, which take part in no validation.
    #[validate(skip)]
    samples: [f64; LENGTH],
}

fn main() {
    let short = Window::<2>::new(WindowDraft {
        index: 2,
        samples: [1.0, 2.0],
    })
    .expect("2 is inside the range of a window of two samples");
    assert_eq!(
        short.index(),
        2,
        "The const parameter must be the upper bound of the range"
    );
    assert_eq!(
        short.samples().len(),
        2,
        "A skipped field of the length of the const parameter must be passed through"
    );

    assert_eq!(
        Window::<2>::new(WindowDraft {
            index: 3,
            samples: [1.0, 2.0],
        })
        .err(),
        Some(WindowValidationError::OutOfRange {
            field: WindowField::Index,
            range: "[0, LENGTH]",
        }),
        "An index above the const parameter must be rejected"
    );

    let long = Window::<4>::new(WindowDraft {
        index: 4,
        samples: [1.0; 4],
    })
    .expect("4 is inside the range of a window of four samples");
    assert_eq!(
        long.index(),
        4,
        "A second const argument must give the same validation"
    );

    assert!(
        Window::<4>::new(WindowDraft {
            index: 5,
            samples: [1.0; 4],
        })
        .is_err(),
        "An index above a longer const parameter must be rejected"
    );
}
