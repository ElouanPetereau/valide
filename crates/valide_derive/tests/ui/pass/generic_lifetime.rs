//! A validated type with a lifetime parameter, which its skipped field borrows.

use valide::{Patch as _, Validate as _};

/// Sample tagged with a borrowed sensor name.
#[derive(Clone, valide_derive::Validate, valide_derive::Patch)]
struct Sample<'sensor> {
    /// Sensor that produced the sample, which takes part in no validation.
    #[validate(skip)]
    sensor: &'sensor str,
    /// Value of the sample.
    #[validate(range(0.0..=1.0))]
    value: f64,
}

fn main() {
    let port = String::from("port sensor");
    let starboard = String::from("starboard sensor");

    let mut sample = Sample::new(SampleDraft {
        sensor: &port,
        value: 0.5,
    })
    .expect("0.5 is inside the range");
    assert_eq!(
        *sample.sensor(),
        "port sensor",
        "A borrowed field must be passed through"
    );
    assert!(
        (sample.value() - 0.5).abs() < f64::EPSILON,
        "The getter must return the built value"
    );

    assert_eq!(
        Sample::new(SampleDraft {
            sensor: &port,
            value: 1.5,
        })
        .err(),
        Some(SampleValidationError::OutOfRange {
            field: SampleField::Value,
            range: "[0.0, 1.0]",
        }),
        "A value above the range must be rejected"
    );

    sample.set_sensor(&starboard);
    assert_eq!(
        *sample.sensor(),
        "starboard sensor",
        "The setter of a skipped field must store the new borrow"
    );

    assert!(
        sample.set_value(0.75).is_ok(),
        "A valid patch must be accepted"
    );
    assert!(
        sample.set_value(2.0).is_err(),
        "An invalid patch must be rejected"
    );
    assert!(
        (sample.value() - 0.75).abs() < f64::EPSILON,
        "A rejected patch must leave the value untouched"
    );

    let restored = Sample::from_draft(sample.to_draft()).expect("The draft stays valid");
    assert_eq!(
        *restored.sensor(),
        "starboard sensor",
        "The draft round trip must keep the borrow"
    );
}
