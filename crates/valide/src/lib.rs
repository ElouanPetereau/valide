#![doc = include_str!("../../../OVERVIEW.md")]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(
    feature = "derive",
    expect(
        clippy::pub_use,
        reason = "the derive macros belong next to the traits they implement"
    )
)]

// The generated code names the crate of the two traits with an absolute path, so the crate must
// reach itself under its own name when it derives its own test model
#[cfg(test)]
extern crate self as valide;

#[cfg(feature = "derive")]
pub use valide_derive::{Patch, Validate};

/// A type that can only be built from a validated draft.
pub trait Validate: Sized {
    /// Unvalidated draft mirror of the type.
    type Draft;
    /// Error that an invalid draft produces.
    type Error;

    /// Validate the given `draft` with a fail fast policy.
    fn validate(draft: &Self::Draft) -> Result<(), Self::Error>;

    /// Build the type from the given `draft` and skip the validation.
    /// The function is meant for the generated code, which calls it after a successful validation.
    /// It enforces no rule of the type, so a direct call on an invalid `draft` builds an invalid
    /// value.
    #[doc(hidden)]
    fn from_draft_unchecked(draft: Self::Draft) -> Self;

    /// Validate the given `draft`, then build the type from it.
    /// Return the first error found in the `draft`.
    fn from_draft(draft: Self::Draft) -> Result<Self, Self::Error> {
        Self::validate(&draft)?;
        Ok(Self::from_draft_unchecked(draft))
    }
}

/// A type that can be patched field by field with validated setters.
pub trait Patch: Validate {
    /// Convert the value back to a draft to patch it.
    fn to_draft(&self) -> Self::Draft;
}

#[cfg(test)]
#[path = "../examples/spacecraft"] // The domain model of the tests is the model of the `spacecraft` example.
mod tests {
    use nalgebra::RealField;
    use num_traits::Float;

    use self::model::{
        CelestialBodyDraft, CelestialBodyKind, CelestialBodyKindDraft,
        InertiaMatrixSerializableDraft, InertiaMatrixSerializableField,
        InertiaMatrixSerializableValidationError, ShadowFractionDraft, SpacecraftDraft,
    };

    /// Domain model that exercises the two derives of the crate.
    #[expect(
        unreachable_pub,
        reason = "the model items are public inside a private test module"
    )]
    #[path = "model.rs"]
    mod model;

    /// Valid inertia draft with a diagonal of 2.0, 3.0 and 4.0 kg·m².
    const VALID_INERTIA_DRAFT: InertiaMatrixSerializableDraft<f64> =
        diagonal_inertia_draft(2.0, 3.0, 4.0);

    /// Standard gravitational parameter of the Earth in cubic meters per second squared (m³/s²).
    const EARTH_GRAVITATIONAL_PARAMETER: f64 = 3.986_004_418e14;

    /// Valid celestial body draft carrying the gravitational parameter of the Earth.
    const VALID_CELESTIAL_BODY_DRAFT: CelestialBodyDraft = CelestialBodyDraft {
        gravitational_parameter: EARTH_GRAVITATIONAL_PARAMETER,
    };

    /// Valid spacecraft draft of 1000.0 kg with a 600.0 kg bus, a 300.0 kg sail
    /// and a 12.5 m2 area.
    const VALID_SPACECRAFT_DRAFT: SpacecraftDraft =
        spacecraft_draft_with_masses(1000.0, 600.0, 300.0);

    /// The three diagonal field cases of an inertia draft.
    const DIAGONAL_INERTIA_FIELD_CASES: [InertiaFieldCase; 3] = [
        (
            "xx",
            |draft, value| draft.xx = value,
            InertiaMatrixSerializableDraft::validate_xx,
            InertiaMatrixSerializableField::Xx,
        ),
        (
            "yy",
            |draft, value| draft.yy = value,
            InertiaMatrixSerializableDraft::validate_yy,
            InertiaMatrixSerializableField::Yy,
        ),
        (
            "zz",
            |draft, value| draft.zz = value,
            InertiaMatrixSerializableDraft::validate_zz,
            InertiaMatrixSerializableField::Zz,
        ),
    ];

    /// The six off-diagonal field cases of an inertia draft.
    const OFF_DIAGONAL_INERTIA_FIELD_CASES: [InertiaFieldCase; 6] = [
        (
            "xy",
            |draft, value| draft.xy = value,
            InertiaMatrixSerializableDraft::validate_xy,
            InertiaMatrixSerializableField::Xy,
        ),
        (
            "xz",
            |draft, value| draft.xz = value,
            InertiaMatrixSerializableDraft::validate_xz,
            InertiaMatrixSerializableField::Xz,
        ),
        (
            "yx",
            |draft, value| draft.yx = value,
            InertiaMatrixSerializableDraft::validate_yx,
            InertiaMatrixSerializableField::Yx,
        ),
        (
            "yz",
            |draft, value| draft.yz = value,
            InertiaMatrixSerializableDraft::validate_yz,
            InertiaMatrixSerializableField::Yz,
        ),
        (
            "zx",
            |draft, value| draft.zx = value,
            InertiaMatrixSerializableDraft::validate_zx,
            InertiaMatrixSerializableField::Zx,
        ),
        (
            "zy",
            |draft, value| draft.zy = value,
            InertiaMatrixSerializableDraft::validate_zy,
            InertiaMatrixSerializableField::Zy,
        ),
    ];

    /// Name, setter, validator and field enum variant of one inertia draft field.
    /// Each case exercises one field validator.
    type InertiaFieldCase = (
        &'static str,
        fn(&mut InertiaMatrixSerializableDraft<f64>, f64),
        fn(
            &InertiaMatrixSerializableDraft<f64>,
        ) -> Result<(), InertiaMatrixSerializableValidationError>,
        InertiaMatrixSerializableField,
    );

    /// Build an inertia draft with the given `xx`, `yy` and `zz` diagonal and zero off-diagonals.
    const fn diagonal_inertia_draft(
        xx: f64,
        yy: f64,
        zz: f64,
    ) -> InertiaMatrixSerializableDraft<f64> {
        InertiaMatrixSerializableDraft {
            xx,
            xy: 0.0,
            xz: 0.0,
            yx: 0.0,
            yy,
            yz: 0.0,
            zx: 0.0,
            zy: 0.0,
            zz,
        }
    }

    /// Build an inertia draft with the given `xx`, `yy` and `zz` diagonal and zero off-diagonals.
    /// The zero of the parameter comes from a trait function, which a constant function cannot call,
    /// so this builder is a plain function while [`diagonal_inertia_draft`] stays a constant one.
    fn diagonal_inertia_draft_with_precision<Type: RealField + Float>(
        xx: Type,
        yy: Type,
        zz: Type,
    ) -> InertiaMatrixSerializableDraft<Type> {
        InertiaMatrixSerializableDraft {
            xx,
            xy: Type::zero(),
            xz: Type::zero(),
            yx: Type::zero(),
            yy,
            yz: Type::zero(),
            zx: Type::zero(),
            zy: Type::zero(),
            zz,
        }
    }

    /// Build an otherwise valid spacecraft draft with the given `mass`, `bus_mass` and `sail_mass`.
    const fn spacecraft_draft_with_masses(
        mass: f64,
        bus_mass: f64,
        sail_mass: f64,
    ) -> SpacecraftDraft {
        SpacecraftDraft {
            mass,
            bus_mass,
            sail_mass,
            area: 12.5,
            inertia_matrix: VALID_INERTIA_DRAFT,
            sun_shadow_fraction: ShadowFractionDraft(0.5),
            primary_orbited_body: CelestialBodyKindDraft::Earth,
        }
    }

    /// Build an otherwise valid spacecraft draft with the given `area` in square meters (m2).
    /// The area carries the precision of the returned draft, so the function reaches every
    /// precision that the parameter of the spacecraft accepts.
    /// Every other field matches the standard valid spacecraft draft.
    fn spacecraft_draft_with_area<Type: RealField + Float>(area: Type) -> SpacecraftDraft<Type> {
        SpacecraftDraft {
            mass: 1000.0,
            bus_mass: 600.0,
            sail_mass: 300.0,
            area,
            inertia_matrix: diagonal_inertia_draft_with_precision(
                Type::from(2.0).expect("2.0 is guaranteed to fit in any Float"),
                Type::from(3.0).expect("3.0 is guaranteed to fit in any Float"),
                Type::from(4.0).expect("4.0 is guaranteed to fit in any Float"),
            ),
            sun_shadow_fraction: ShadowFractionDraft(0.5),
            primary_orbited_body: CelestialBodyKindDraft::Earth,
        }
    }

    /// Assert that `actual` is exactly `expected`.
    /// The `described_as` argument names the compared quantity.
    /// The function compares the bit patterns, so the check stays exact without a float comparison.
    fn assert_float_eq(actual: f64, expected: f64, described_as: &str) {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "The {described_as} must be exactly {expected} but was {actual}"
        );
    }

    /// Return the gravitational parameter of the given `celestial_body_kind` in m³/s².
    /// The kind must be a custom body, which is the only kind that carries a payload.
    fn custom_gravitational_parameter(celestial_body_kind: &CelestialBodyKind) -> f64 {
        match celestial_body_kind {
            CelestialBodyKind::Custom(celestial_body) => celestial_body.gravitational_parameter(),
            CelestialBodyKind::Sun | CelestialBodyKind::Earth => {
                panic!("The tested celestial body kind must be a custom body")
            }
        }
    }

    /// Write `new_value` in the field that `set_field` selects, on an otherwise valid draft.
    /// Return the result of the matching field validator `validate_field`.
    fn validate_inertia_field(
        set_field: fn(&mut InertiaMatrixSerializableDraft<f64>, f64),
        validate_field: fn(
            &InertiaMatrixSerializableDraft<f64>,
        ) -> Result<(), InertiaMatrixSerializableValidationError>,
        new_value: f64,
    ) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut draft = VALID_INERTIA_DRAFT;
        set_field(&mut draft, new_value);
        validate_field(&draft)
    }

    /// Field validators of every validated type.
    mod field_validation {
        /// Bounds of the shadow fraction value.
        mod shadow_fraction {
            use crate::tests::model::{
                ShadowFractionDraft, ShadowFractionField, ShadowFractionValidationError,
            };

            #[test]
            fn accepts_lower_bound() {
                assert_eq!(
                    ShadowFractionDraft(0.0).validate_value(),
                    Ok(()),
                    "The lower bound 0.0 must be accepted"
                );
            }

            #[test]
            fn accepts_upper_bound() {
                assert_eq!(
                    ShadowFractionDraft(1.0).validate_value(),
                    Ok(()),
                    "The upper bound 1.0 must be accepted"
                );
            }

            #[test]
            fn accepts_interior_value() {
                assert_eq!(
                    ShadowFractionDraft(0.5).validate_value(),
                    Ok(()),
                    "An interior value must be accepted"
                );
            }

            #[test]
            fn accepts_negative_zero() {
                assert_eq!(
                    ShadowFractionDraft(-0.0).validate_value(),
                    Ok(()),
                    "The negative zero must be accepted since it compares equal to 0.0"
                );
            }

            #[test]
            fn rejects_below_lower_bound() {
                assert_eq!(
                    ShadowFractionDraft(-0.1).validate_value(),
                    Err(ShadowFractionValidationError::OutOfRange {
                        field: ShadowFractionField::Value,
                        range: "[0.0, 1.0]",
                    }),
                    "A value below the lower bound must be rejected"
                );
            }

            #[test]
            fn rejects_above_upper_bound() {
                for rejected_value in [1.000_001, 1.0 + f64::EPSILON] {
                    assert_eq!(
                        ShadowFractionDraft(rejected_value).validate_value(),
                        Err(ShadowFractionValidationError::OutOfRange {
                            field: ShadowFractionField::Value,
                            range: "[0.0, 1.0]",
                        }),
                        "The value {rejected_value} above the upper bound must be rejected"
                    );
                }
            }

            #[test]
            fn rejects_nan() {
                assert_eq!(
                    ShadowFractionDraft(f64::NAN).validate_value(),
                    Err(ShadowFractionValidationError::OutOfRange {
                        field: ShadowFractionField::Value,
                        range: "[0.0, 1.0]",
                    }),
                    "A not a number value must be rejected"
                );
            }

            #[test]
            fn rejects_positive_infinity() {
                assert_eq!(
                    ShadowFractionDraft(f64::INFINITY).validate_value(),
                    Err(ShadowFractionValidationError::OutOfRange {
                        field: ShadowFractionField::Value,
                        range: "[0.0, 1.0]",
                    }),
                    "The positive infinity must be rejected"
                );
            }
        }

        /// Bounds of the gravitational parameter of a celestial body.
        mod celestial_body {
            use crate::tests::{
                VALID_CELESTIAL_BODY_DRAFT,
                model::{CelestialBodyDraft, CelestialBodyField, CelestialBodyValidationError},
            };

            #[test]
            fn accepts_the_gravitational_parameter_of_the_earth() {
                assert_eq!(
                    VALID_CELESTIAL_BODY_DRAFT.validate_gravitational_parameter(),
                    Ok(()),
                    "The gravitational parameter of the Earth must be accepted"
                );
            }

            #[test]
            fn accepts_smallest_positive() {
                assert_eq!(
                    CelestialBodyDraft {
                        gravitational_parameter: (f64::MIN_POSITIVE)
                    }
                    .validate_gravitational_parameter(),
                    Ok(()),
                    "The smallest positive gravitational parameter must be accepted"
                );
            }

            #[test]
            fn rejects_zero_negative_infinity_and_nan() {
                for rejected_value in [0.0, -1.0, f64::INFINITY, f64::NAN] {
                    assert_eq!(
                        CelestialBodyDraft {
                            gravitational_parameter: rejected_value
                        }
                        .validate_gravitational_parameter(),
                        Err(CelestialBodyValidationError::OutOfRange {
                            field: CelestialBodyField::GravitationalParameter,
                            range: "]0.0, +inf[",
                        }),
                        "The gravitational parameter {rejected_value} must be rejected"
                    );
                }
            }
        }

        /// Bounds of the three diagonal inertia entries, valid over the open range ]0, +inf[.
        mod inertia_matrix_diagonal {
            use crate::tests::{
                DIAGONAL_INERTIA_FIELD_CASES, diagonal_inertia_draft_with_precision,
                model::{InertiaMatrixSerializableField, InertiaMatrixSerializableValidationError},
                validate_inertia_field,
            };

            #[test]
            fn rejects_zero() {
                for (field_name, set_field, validate_field, expected_field) in
                    DIAGONAL_INERTIA_FIELD_CASES
                {
                    assert_eq!(
                        validate_inertia_field(set_field, validate_field, 0.0),
                        Err(InertiaMatrixSerializableValidationError::OutOfRange {
                            field: expected_field,
                            range: "]Type::zero(), Type::infinity()[",
                        }),
                        "The excluded lower bound 0.0 must be rejected for the {field_name} field"
                    );
                }
            }

            #[test]
            fn accepts_smallest_positive() {
                for (field_name, set_field, validate_field, ..) in DIAGONAL_INERTIA_FIELD_CASES {
                    assert_eq!(
                        validate_inertia_field(set_field, validate_field, f64::MIN_POSITIVE),
                        Ok(()),
                        "The smallest positive value must be accepted for the {field_name} field"
                    );
                }
            }

            #[test]
            fn accepts_max_finite() {
                for (field_name, set_field, validate_field, ..) in DIAGONAL_INERTIA_FIELD_CASES {
                    assert_eq!(
                        validate_inertia_field(set_field, validate_field, f64::MAX),
                        Ok(()),
                        "The largest finite value must be accepted for the {field_name} field"
                    );
                }
            }

            #[test]
            fn rejects_negative() {
                for (field_name, set_field, validate_field, expected_field) in
                    DIAGONAL_INERTIA_FIELD_CASES
                {
                    assert_eq!(
                        validate_inertia_field(set_field, validate_field, -1.0),
                        Err(InertiaMatrixSerializableValidationError::OutOfRange {
                            field: expected_field,
                            range: "]Type::zero(), Type::infinity()[",
                        }),
                        "A negative value must be rejected for the {field_name} field"
                    );
                }
            }

            #[test]
            fn rejects_positive_infinity_at_f64_precision() {
                for (field_name, set_field, validate_field, expected_field) in
                    DIAGONAL_INERTIA_FIELD_CASES
                {
                    assert_eq!(
                        validate_inertia_field(set_field, validate_field, f64::INFINITY),
                        Err(InertiaMatrixSerializableValidationError::OutOfRange {
                            field: expected_field,
                            range: "]Type::zero(), Type::infinity()[",
                        }),
                        "The excluded positive infinity must be rejected for the {field_name} field"
                    );
                }
            }

            #[test]
            fn rejects_positive_infinity_at_f32_precision() {
                let draft = diagonal_inertia_draft_with_precision(2.0_f32, f32::INFINITY, 4.0_f32);

                assert_eq!(
                    draft.validate_yy(),
                    Err(InertiaMatrixSerializableValidationError::OutOfRange {
                        field: InertiaMatrixSerializableField::Yy,
                        range: "]Type::zero(), Type::infinity()[",
                    }),
                    "The excluded positive infinity must be rejected at the single precision"
                );
            }

            #[test]
            fn rejects_nan_at_f64_precision() {
                for (field_name, set_field, validate_field, expected_field) in
                    DIAGONAL_INERTIA_FIELD_CASES
                {
                    assert_eq!(
                        validate_inertia_field(set_field, validate_field, f64::NAN),
                        Err(InertiaMatrixSerializableValidationError::OutOfRange {
                            field: expected_field,
                            range: "]Type::zero(), Type::infinity()[",
                        }),
                        "A not a number value must be rejected for the {field_name} field"
                    );
                }
            }

            #[test]
            fn rejects_nan_at_f32_precision() {
                let draft = diagonal_inertia_draft_with_precision(f32::NAN, 3.0_f32, 4.0_f32);

                assert_eq!(
                    draft.validate_xx(),
                    Err(InertiaMatrixSerializableValidationError::OutOfRange {
                        field: InertiaMatrixSerializableField::Xx,
                        range: "]Type::zero(), Type::infinity()[",
                    }),
                    "A not a number diagonal entry must be rejected at the single precision"
                );
            }
        }

        /// Finiteness of the six off-diagonal inertia entries.
        mod inertia_matrix_off_diagonal {
            use crate::tests::{
                OFF_DIAGONAL_INERTIA_FIELD_CASES, diagonal_inertia_draft_with_precision,
                model::{InertiaMatrixSerializableField, InertiaMatrixSerializableValidationError},
                validate_inertia_field,
            };

            #[test]
            fn accepts_zero_negative_and_large_finite() {
                for (field_name, set_field, validate_field, ..) in OFF_DIAGONAL_INERTIA_FIELD_CASES
                {
                    for accepted_value in [0.0, -5.0, f64::MAX] {
                        assert_eq!(
                            validate_inertia_field(set_field, validate_field, accepted_value),
                            Ok(()),
                            "The value {accepted_value} must be accepted for the {field_name} field"
                        );
                    }
                }
            }

            #[test]
            fn rejects_nan_at_f64_precision() {
                for (field_name, set_field, validate_field, expected_field) in
                    OFF_DIAGONAL_INERTIA_FIELD_CASES
                {
                    assert_eq!(
                        validate_inertia_field(set_field, validate_field, f64::NAN),
                        Err(InertiaMatrixSerializableValidationError::NotFinite {
                            field: expected_field,
                        }),
                        "A not a number value must be rejected for the {field_name} field"
                    );
                }
            }

            #[test]
            fn rejects_nan_at_f32_precision() {
                let mut draft = diagonal_inertia_draft_with_precision(2.0_f32, 3.0_f32, 4.0_f32);
                draft.xy = f32::NAN;

                assert_eq!(
                    draft.validate_xy(),
                    Err(InertiaMatrixSerializableValidationError::NotFinite {
                        field: InertiaMatrixSerializableField::Xy,
                    }),
                    "A not a number off-diagonal entry must be rejected at the single precision"
                );
            }

            #[test]
            fn rejects_positive_infinity() {
                for (field_name, set_field, validate_field, expected_field) in
                    OFF_DIAGONAL_INERTIA_FIELD_CASES
                {
                    assert_eq!(
                        validate_inertia_field(set_field, validate_field, f64::INFINITY),
                        Err(InertiaMatrixSerializableValidationError::NotFinite {
                            field: expected_field,
                        }),
                        "The positive infinity must be rejected for the {field_name} field"
                    );
                }
            }

            #[test]
            fn rejects_negative_infinity() {
                for (field_name, set_field, validate_field, expected_field) in
                    OFF_DIAGONAL_INERTIA_FIELD_CASES
                {
                    assert_eq!(
                        validate_inertia_field(set_field, validate_field, f64::NEG_INFINITY),
                        Err(InertiaMatrixSerializableValidationError::NotFinite {
                            field: expected_field,
                        }),
                        "The negative infinity must be rejected for the {field_name} field"
                    );
                }
            }
        }

        /// Bounds of the spacecraft masses and wrapper variants of the nested errors.
        mod spacecraft_masses {
            use crate::tests::{
                VALID_SPACECRAFT_DRAFT, diagonal_inertia_draft,
                model::{
                    InertiaMatrixSerializableField, InertiaMatrixSerializableValidationError,
                    ShadowFractionDraft, ShadowFractionField, ShadowFractionValidationError,
                    SpacecraftDraft, SpacecraftField, SpacecraftValidationError,
                },
                spacecraft_draft_with_masses,
            };

            #[test]
            fn mass_accepts_zero() {
                assert_eq!(
                    spacecraft_draft_with_masses(0.0, 0.0, 0.0).validate_mass(),
                    Ok(()),
                    "A total mass of 0.0 kg is inside the range and must be accepted"
                );
            }

            #[test]
            fn mass_rejects_negative() {
                assert_eq!(
                    spacecraft_draft_with_masses(-1.0, 600.0, 300.0).validate_mass(),
                    Err(SpacecraftValidationError::OutOfRange {
                        field: SpacecraftField::Mass,
                        range: "[0.0, +inf[",
                    }),
                    "A negative total mass must be rejected"
                );
            }

            #[test]
            fn mass_rejects_positive_infinity() {
                assert_eq!(
                    spacecraft_draft_with_masses(f64::INFINITY, 600.0, 300.0).validate_mass(),
                    Err(SpacecraftValidationError::OutOfRange {
                        field: SpacecraftField::Mass,
                        range: "[0.0, +inf[",
                    }),
                    "The excluded positive infinity must be rejected as a total mass"
                );
            }

            #[test]
            fn mass_rejects_nan() {
                assert_eq!(
                    spacecraft_draft_with_masses(f64::NAN, 600.0, 300.0).validate_mass(),
                    Err(SpacecraftValidationError::OutOfRange {
                        field: SpacecraftField::Mass,
                        range: "[0.0, +inf[",
                    }),
                    "A not a number total mass must be rejected"
                );
            }

            #[test]
            fn bus_mass_accepts_upper_bound() {
                assert_eq!(
                    spacecraft_draft_with_masses(1000.0, 10_000.0, 300.0).validate_bus_mass(),
                    Ok(()),
                    "The included upper bound of the bus mass must be accepted"
                );
            }

            #[test]
            fn bus_mass_rejects_above_upper_bound() {
                assert_eq!(
                    spacecraft_draft_with_masses(1000.0, 10_000.1, 300.0).validate_bus_mass(),
                    Err(SpacecraftValidationError::OutOfRange {
                        field: SpacecraftField::BusMass,
                        range: "]0.0, 10_000.0]",
                    }),
                    "A bus mass above the upper bound must be rejected"
                );
            }

            #[test]
            fn bus_mass_rejects_zero() {
                assert_eq!(
                    spacecraft_draft_with_masses(1000.0, 0.0, 300.0).validate_bus_mass(),
                    Err(SpacecraftValidationError::OutOfRange {
                        field: SpacecraftField::BusMass,
                        range: "]0.0, 10_000.0]",
                    }),
                    "The excluded lower bound of the bus mass must be rejected"
                );
            }

            #[test]
            fn bus_mass_rejects_negative() {
                assert_eq!(
                    spacecraft_draft_with_masses(1000.0, -1.0, 300.0).validate_bus_mass(),
                    Err(SpacecraftValidationError::OutOfRange {
                        field: SpacecraftField::BusMass,
                        range: "]0.0, 10_000.0]",
                    }),
                    "A negative bus mass must be rejected"
                );
            }

            #[test]
            fn sail_mass_accepts_upper_bound() {
                assert_eq!(
                    spacecraft_draft_with_masses(1000.0, 600.0, 10_000.0).validate_sail_mass(),
                    Ok(()),
                    "The included upper bound of the sail mass must be accepted"
                );
            }

            #[test]
            fn sail_mass_rejects_above_upper_bound() {
                assert_eq!(
                    spacecraft_draft_with_masses(1000.0, 600.0, 10_000.1).validate_sail_mass(),
                    Err(SpacecraftValidationError::OutOfRange {
                        field: SpacecraftField::SailMass,
                        range: "[0.0, 10_000.0]",
                    }),
                    "A sail mass above the upper bound must be rejected"
                );
            }

            #[test]
            fn sail_mass_rejects_negative() {
                assert_eq!(
                    spacecraft_draft_with_masses(1000.0, 600.0, -1.0).validate_sail_mass(),
                    Err(SpacecraftValidationError::OutOfRange {
                        field: SpacecraftField::SailMass,
                        range: "[0.0, 10_000.0]",
                    }),
                    "A negative sail mass must be rejected"
                );
            }

            #[test]
            fn inertia_field_wraps_nested_error() {
                let draft = SpacecraftDraft {
                    inertia_matrix: diagonal_inertia_draft(0.0, 3.0, 4.0),
                    ..VALID_SPACECRAFT_DRAFT
                };

                assert_eq!(
                    draft.validate_inertia_matrix(),
                    Err(SpacecraftValidationError::InertiaMatrixValidationError(
                        InertiaMatrixSerializableValidationError::OutOfRange {
                            field: InertiaMatrixSerializableField::Xx,
                            range: "]Type::zero(), Type::infinity()[",
                        }
                    )),
                    "The nested inertia matrix error must be wrapped by the spacecraft error"
                );
            }

            #[test]
            fn shadow_field_wraps_nested_error() {
                let draft = SpacecraftDraft {
                    sun_shadow_fraction: ShadowFractionDraft(1.5),
                    ..VALID_SPACECRAFT_DRAFT
                };

                assert_eq!(
                    draft.validate_sun_shadow_fraction(),
                    Err(SpacecraftValidationError::SunShadowFractionValidationError(
                        ShadowFractionValidationError::OutOfRange {
                            field: ShadowFractionField::Value,
                            range: "[0.0, 1.0]",
                        }
                    )),
                    "The nested shadow fraction error must be wrapped by the spacecraft error"
                );
            }
        }

        /// Order guarantees of the fail fast validation.
        mod fail_fast_order {
            use crate::tests::{
                VALID_INERTIA_DRAFT, VALID_SPACECRAFT_DRAFT, diagonal_inertia_draft,
                model::{
                    InertiaMatrixSerializableField, InertiaMatrixSerializableValidationError,
                    SpacecraftDraft, SpacecraftField, SpacecraftValidationError,
                },
            };

            #[test]
            fn first_declared_field_error_wins() {
                let mut draft = VALID_INERTIA_DRAFT;
                draft.xx = 0.0;
                draft.xy = f64::NAN;

                assert_eq!(
                    draft.validate(),
                    Err(InertiaMatrixSerializableValidationError::OutOfRange {
                        field: InertiaMatrixSerializableField::Xx,
                        range: "]Type::zero(), Type::infinity()[",
                    }),
                    "The xx field is declared first so its error must be reported"
                );
            }

            #[test]
            fn field_validators_run_before_final_validation() {
                let mut draft = VALID_INERTIA_DRAFT;
                draft.xy = f64::NAN;
                // The mirrored entry stays at zero so the pair is also asymmetric
                draft.yx = 0.0;

                assert_eq!(
                    draft.validate(),
                    Err(InertiaMatrixSerializableValidationError::NotFinite {
                        field: InertiaMatrixSerializableField::Xy,
                    }),
                    "The field validators must run before the realizability validation"
                );
            }

            #[test]
            fn spacecraft_field_order() {
                let draft = SpacecraftDraft {
                    mass: -1.0,
                    inertia_matrix: diagonal_inertia_draft(0.0, 3.0, 4.0),
                    ..VALID_SPACECRAFT_DRAFT
                };

                assert_eq!(
                    draft.validate(),
                    Err(SpacecraftValidationError::OutOfRange {
                        field: SpacecraftField::Mass,
                        range: "[0.0, +inf[",
                    }),
                    "The mass field is declared first so its error must be reported"
                );
            }
        }
    }

    /// Delegation of a validated enum to the payload of its variants.
    mod variant_validation {
        use crate::tests::{
            VALID_CELESTIAL_BODY_DRAFT,
            model::{
                CelestialBodyDraft, CelestialBodyField, CelestialBodyKindDraft,
                CelestialBodyKindValidationError, CelestialBodyValidationError,
            },
        };

        #[test]
        fn accepts_every_unit_variant() {
            for (variant_name, draft) in [
                ("Sun", CelestialBodyKindDraft::Sun),
                ("Earth", CelestialBodyKindDraft::Earth),
            ] {
                assert_eq!(
                    draft.validate(),
                    Ok(()),
                    "The {variant_name} variant carries no rule, so it must always be valid"
                );
            }
        }

        #[test]
        fn accepts_a_valid_custom_body() {
            assert_eq!(
                CelestialBodyKindDraft::Custom(VALID_CELESTIAL_BODY_DRAFT).validate(),
                Ok(()),
                "A custom body with a valid gravitational parameter must be accepted"
            );
        }

        #[test]
        fn wraps_the_error_of_a_rejected_custom_body() {
            assert_eq!(
                CelestialBodyKindDraft::Custom(CelestialBodyDraft {
                    gravitational_parameter: 0.0
                })
                .validate(),
                Err(CelestialBodyKindValidationError::CustomValidationError(
                    CelestialBodyValidationError::OutOfRange {
                        field: CelestialBodyField::GravitationalParameter,
                        range: "]0.0, +inf[",
                    }
                )),
                "The custom variant must wrap the error of its own payload type"
            );
        }
    }

    /// Direct calls of the final validations on hand built drafts.
    mod final_validation {
        /// Symmetry and physical realizability of an inertia matrix draft.
        mod realizability {
            use crate::tests::{
                VALID_INERTIA_DRAFT, diagonal_inertia_draft, diagonal_inertia_draft_with_precision,
                model::{InertiaMatrixRealizabilityValidationError, InertiaMatrixSerializable},
            };

            #[test]
            fn accepts_spherical_inertia() {
                assert_eq!(
                    InertiaMatrixSerializable::validate_realizability(&diagonal_inertia_draft(
                        2.0, 2.0, 2.0
                    )),
                    Ok(()),
                    "A spherical inertia must be realizable"
                );
            }

            #[test]
            fn accepts_asymmetric_diagonal() {
                assert_eq!(
                    InertiaMatrixSerializable::validate_realizability(&VALID_INERTIA_DRAFT),
                    Ok(()),
                    "A diagonal satisfying the triangle inequalities must be realizable"
                );
            }

            #[test]
            fn rejects_asymmetric_pair() {
                let mut draft = VALID_INERTIA_DRAFT;
                draft.xy = 1.0;
                draft.yx = 0.0;

                assert_eq!(
                    InertiaMatrixSerializable::validate_realizability(&draft),
                    Err(InertiaMatrixRealizabilityValidationError::NotSymmetric),
                    "An off-diagonal pair differing by more than the tolerance must be rejected"
                );
            }

            #[test]
            fn accepts_asymmetry_at_tolerance() {
                let mut draft = VALID_INERTIA_DRAFT;
                draft.xy = InertiaMatrixSerializable::<f64>::SYMMETRY_TOLERANCE;
                draft.yx = 0.0;

                assert_eq!(
                    InertiaMatrixSerializable::validate_realizability(&draft),
                    Ok(()),
                    "A mismatch exactly equal to the symmetry tolerance must be accepted"
                );
            }

            #[test]
            fn rejects_negative_trace() {
                assert_eq!(
                    InertiaMatrixSerializable::validate_realizability(&diagonal_inertia_draft(
                        -1.0, -1.0, -1.0
                    )),
                    Err(InertiaMatrixRealizabilityValidationError::NegativeTrace),
                    "A negative trace must be rejected"
                );
            }

            #[test]
            fn rejects_triangle_inequality_violation_at_f64_precision() {
                assert_eq!(
                    InertiaMatrixSerializable::validate_realizability(&diagonal_inertia_draft(
                        1.0, 1.0, 3.0
                    )),
                    Err(
                        InertiaMatrixRealizabilityValidationError::CovarianceNotPositiveSemiDefinite
                    ),
                    "A diagonal violating Ixx + Iyy >= Izz must be rejected"
                );
            }

            #[test]
            fn rejects_triangle_inequality_violation_at_f32_precision() {
                assert_eq!(
                    InertiaMatrixSerializable::validate_realizability(
                        &diagonal_inertia_draft_with_precision(1.0_f32, 1.0_f32, 3.0_f32)
                    ),
                    Err(
                        InertiaMatrixRealizabilityValidationError::CovarianceNotPositiveSemiDefinite
                    ),
                    "A single precision diagonal violating Ixx + Iyy >= Izz must be rejected"
                );
            }

            #[test]
            fn accepts_degenerate_planar_body() {
                assert_eq!(
                    InertiaMatrixSerializable::validate_realizability(&diagonal_inertia_draft(
                        1.0, 1.0, 2.0
                    )),
                    Ok(()),
                    "A planar body sitting exactly on the semi-definite boundary must be accepted"
                );
            }

            #[test]
            fn rejects_order_two_minor_violation() {
                let mut draft = diagonal_inertia_draft(2.0, 2.0, 2.0);
                draft.xy = 2.5;
                draft.yx = 2.5;

                assert_eq!(
                    InertiaMatrixSerializable::validate_realizability(&draft),
                    Err(
                        InertiaMatrixRealizabilityValidationError::CovarianceNotPositiveSemiDefinite
                    ),
                    "A negative order two principal minor must be rejected"
                );
            }

            #[test]
            fn rejects_determinant_only_violation() {
                let mut draft = diagonal_inertia_draft(2.0, 2.0, 2.0);
                draft.xy = 0.9;
                draft.xz = 0.9;
                draft.yx = 0.9;
                draft.yz = 0.9;
                draft.zx = 0.9;
                draft.zy = 0.9;

                // Covariance sigma = half_trace * identity - inertia = [[1, -0.9, -0.9],
                // [-0.9, 1, -0.9], [-0.9, -0.9, 1]], so the order one minors are 1.0, the order
                // two minors are 0.19 and only the determinant, -2.888, is negative.
                assert_eq!(
                    InertiaMatrixSerializable::validate_realizability(&draft),
                    Err(
                        InertiaMatrixRealizabilityValidationError::CovarianceNotPositiveSemiDefinite
                    ),
                    "Sylvester's criterion needs the determinant on top of the smaller minors"
                );
            }

            #[test]
            fn accepts_psd_violation_within_tolerance() {
                assert_eq!(
                    InertiaMatrixSerializable::validate_realizability(&diagonal_inertia_draft(
                        1.0,
                        1.0,
                        2.0 + 1e-12
                    )),
                    Ok(()),
                    "A semi-definiteness violation below the tolerance must be absorbed"
                );
            }

            #[test]
            fn nan_reports_as_not_symmetric_when_called_standalone() {
                let mut draft = VALID_INERTIA_DRAFT;
                draft.xy = f64::NAN;
                draft.yx = 0.0;

                // Pins the documented precondition of the function, the entries are expected to be
                // finite because the per field validators run first and are bypassed here.
                assert_eq!(
                    InertiaMatrixSerializable::validate_realizability(&draft),
                    Err(InertiaMatrixRealizabilityValidationError::NotSymmetric),
                    "A not a number entry must surface as a symmetry failure when called alone"
                );
            }
        }

        /// Tolerance semantics of the spacecraft mass sum.
        mod mass_sum {
            use crate::tests::{
                model::{Spacecraft, SpacecraftMassSumValidationError},
                spacecraft_draft_with_masses,
            };

            #[test]
            fn accepts_exact_equality() {
                assert_eq!(
                    Spacecraft::validate_mass_sum(&spacecraft_draft_with_masses(
                        900.0, 600.0, 300.0
                    )),
                    Ok(()),
                    "A total mass exactly equal to the sum of the parts must be accepted"
                );
            }

            #[test]
            fn accepts_within_tolerance() {
                assert_eq!(
                    Spacecraft::validate_mass_sum(&spacecraft_draft_with_masses(
                        3.0 - 0.5e-9,
                        1.0,
                        2.0
                    )),
                    Ok(()),
                    "A deficit smaller than the mass sum tolerance must be accepted"
                );
            }

            #[test]
            fn rejects_below_tolerance() {
                assert_eq!(
                    Spacecraft::validate_mass_sum(&spacecraft_draft_with_masses(
                        3.0 - 2e-9,
                        1.0,
                        2.0
                    )),
                    Err(SpacecraftMassSumValidationError::MassSmallerThanSum),
                    "A deficit larger than the mass sum tolerance must be rejected"
                );
            }

            #[test]
            fn accepts_above_sum() {
                assert_eq!(
                    Spacecraft::validate_mass_sum(&spacecraft_draft_with_masses(
                        1000.0, 600.0, 300.0
                    )),
                    Ok(()),
                    "A total mass above the sum of the parts must be accepted"
                );
            }

            #[test]
            fn float_representation_sum_passes() {
                // 0.1 + 0.2 exceeds 0.3 by about 5.6e-17, far below the 1e-9 tolerance
                assert_eq!(
                    Spacecraft::validate_mass_sum(&spacecraft_draft_with_masses(0.3, 0.1, 0.2)),
                    Ok(()),
                    "A deficit caused by the float representation must be absorbed"
                );
            }
        }
    }

    /// Construction entry points of the validated types.
    mod construction {
        use core::error::Error as _;

        use crate::{
            Validate as _,
            tests::model::CelestialBodyDraft,
            tests::{
                EARTH_GRAVITATIONAL_PARAMETER, VALID_CELESTIAL_BODY_DRAFT, VALID_INERTIA_DRAFT,
                VALID_SPACECRAFT_DRAFT, assert_float_eq, custom_gravitational_parameter,
                diagonal_inertia_draft, diagonal_inertia_draft_with_precision,
                model::{
                    CelestialBodyField, CelestialBodyKind, CelestialBodyKindDraft,
                    CelestialBodyKindValidationError, CelestialBodyValidationError,
                    InertiaMatrixSerializable, InertiaMatrixSerializableField,
                    InertiaMatrixSerializableValidationError, ShadowFraction, ShadowFractionDraft,
                    ShadowFractionField, ShadowFractionValidationError, Spacecraft,
                    SpacecraftDraft, SpacecraftField, SpacecraftMassSumValidationError,
                    SpacecraftValidationError,
                },
                spacecraft_draft_with_area, spacecraft_draft_with_masses,
            },
        };

        #[test]
        fn serializable_new_accepts_valid_draft_and_preserves_fields() {
            let matrix = InertiaMatrixSerializable::new(VALID_INERTIA_DRAFT)
                .expect("The valid inertia draft must build an inertia matrix");
            let expected = VALID_INERTIA_DRAFT;

            assert_float_eq(*matrix.xx(), expected.xx, "xx field of the built matrix");
            assert_float_eq(*matrix.xy(), expected.xy, "xy field of the built matrix");
            assert_float_eq(*matrix.xz(), expected.xz, "xz field of the built matrix");
            assert_float_eq(*matrix.yx(), expected.yx, "yx field of the built matrix");
            assert_float_eq(*matrix.yy(), expected.yy, "yy field of the built matrix");
            assert_float_eq(*matrix.yz(), expected.yz, "yz field of the built matrix");
            assert_float_eq(*matrix.zx(), expected.zx, "zx field of the built matrix");
            assert_float_eq(*matrix.zy(), expected.zy, "zy field of the built matrix");
            assert_float_eq(*matrix.zz(), expected.zz, "zz field of the built matrix");
        }

        #[test]
        fn serializable_new_returns_first_error() {
            let mut draft = diagonal_inertia_draft(0.0, 3.0, 4.0);
            draft.xy = f64::NAN;

            assert_eq!(
                InertiaMatrixSerializable::new(draft).err(),
                Some(InertiaMatrixSerializableValidationError::OutOfRange {
                    field: InertiaMatrixSerializableField::Xx,
                    range: "]Type::zero(), Type::infinity()[",
                }),
                "The construction must report the first field error found"
            );
        }

        #[test]
        fn shadow_new_valid() {
            let fraction = ShadowFraction::new(ShadowFractionDraft(0.25))
                .expect("The valid shadow fraction draft must build a shadow fraction");

            assert_float_eq(fraction.value(), 0.25, "value of the built shadow fraction");
        }

        #[test]
        fn shadow_new_invalid() {
            assert_eq!(
                ShadowFraction::new(ShadowFractionDraft(1.5)).err(),
                Some(ShadowFractionValidationError::OutOfRange {
                    field: ShadowFractionField::Value,
                    range: "[0.0, 1.0]",
                }),
                "A shadow fraction above the upper bound must not build"
            );
        }

        #[test]
        fn spacecraft_new_valid_full_draft() {
            let spacecraft = Spacecraft::new(VALID_SPACECRAFT_DRAFT)
                .expect("The valid spacecraft draft must build a spacecraft");

            assert_float_eq(spacecraft.mass(), 1000.0, "mass of the built spacecraft");
            assert_float_eq(
                spacecraft.bus_mass(),
                600.0,
                "bus mass of the built spacecraft",
            );
            assert_float_eq(
                spacecraft.sail_mass(),
                300.0,
                "sail mass of the built spacecraft",
            );
            assert_float_eq(*spacecraft.area(), 12.5, "area of the built spacecraft");
            assert_float_eq(
                spacecraft.inertia_matrix().matrix().m11,
                2.0,
                "xx entry of the nested inertia matrix",
            );
            assert_float_eq(
                spacecraft.inertia_matrix().matrix().m33,
                4.0,
                "zz entry of the nested inertia matrix",
            );
            assert_float_eq(
                spacecraft.sun_shadow_fraction().value(),
                0.5,
                "value of the nested shadow fraction",
            );
            assert!(
                matches!(spacecraft.primary_orbited_body(), CelestialBodyKind::Earth),
                "The skipped primary orbited body must be passed through verbatim"
            );
        }

        #[test]
        fn spacecraft_new_rejects_mass_sum_violation() {
            let draft = spacecraft_draft_with_masses(100.0, 600.0, 300.0);

            assert_eq!(
                Spacecraft::new(draft).err(),
                Some(SpacecraftValidationError::MassSumValidationError(
                    SpacecraftMassSumValidationError::MassSmallerThanSum
                )),
                "A total mass smaller than the sum of the parts must not build"
            );
        }

        #[test]
        fn spacecraft_new_accepts_finite_area_at_f64_precision() {
            let spacecraft = Spacecraft::new(spacecraft_draft_with_area(25.0_f64))
                .expect("A finite area must build a spacecraft");

            assert_float_eq(
                *spacecraft.area(),
                25.0,
                "area of the double precision spacecraft",
            );
        }

        #[test]
        fn spacecraft_new_accepts_finite_area_at_f32_precision() {
            let spacecraft = Spacecraft::<f32>::new(spacecraft_draft_with_area(25.0_f32))
                .expect("A finite area must build a single precision spacecraft");

            // The helper of the double precision cannot compare a single precision area,
            // so the check reads the bit pattern of the value directly
            assert_eq!(
                spacecraft.area().to_bits(),
                25.0_f32.to_bits(),
                "The area of the single precision spacecraft must be exactly 25.0 but was {}",
                spacecraft.area()
            );
        }

        #[test]
        fn spacecraft_new_rejects_nan_area_at_f64_precision() {
            assert_eq!(
                Spacecraft::new(spacecraft_draft_with_area(f64::NAN)).err(),
                Some(SpacecraftValidationError::NotFinite {
                    field: SpacecraftField::Area,
                }),
                "A not a number area must be rejected at the double precision"
            );
        }

        #[test]
        fn spacecraft_new_rejects_infinite_area_at_f32_precision() {
            assert_eq!(
                Spacecraft::<f32>::new(spacecraft_draft_with_area(f32::INFINITY)).err(),
                Some(SpacecraftValidationError::NotFinite {
                    field: SpacecraftField::Area,
                }),
                "The positive infinity must be rejected as an area at the single precision"
            );
        }

        #[test]
        fn spacecraft_defaulted_area_parameter_builds() {
            // The two bare names carry the default of the parameter,
            // so they must name the double precision spacecraft and its draft
            let draft: SpacecraftDraft = VALID_SPACECRAFT_DRAFT;
            let spacecraft: Spacecraft =
                Spacecraft::new(draft).expect("The valid spacecraft draft must build a spacecraft");

            assert_float_eq(*spacecraft.area(), 12.5, "area of the defaulted spacecraft");
        }

        #[test]
        fn from_draft_unchecked_skips_validation() {
            let matrix = InertiaMatrixSerializable::from_draft_unchecked(diagonal_inertia_draft(
                0.0, 3.0, 4.0,
            ));

            assert_float_eq(
                *matrix.xx(),
                0.0,
                "xx field built through the unchecked contract",
            );
        }

        #[test]
        fn try_from_equals_from_draft() {
            let try_from_error =
                InertiaMatrixSerializable::try_from(diagonal_inertia_draft(0.0, 3.0, 4.0)).err();
            let from_draft_error =
                InertiaMatrixSerializable::from_draft(diagonal_inertia_draft(0.0, 3.0, 4.0)).err();

            assert_eq!(
                try_from_error, from_draft_error,
                "Both construction paths must report the same error"
            );
        }

        #[test]
        fn a_wrapper_variant_reports_the_error_it_holds_as_its_source() {
            let draft = SpacecraftDraft {
                inertia_matrix: diagonal_inertia_draft(0.0, 3.0, 4.0),
                ..VALID_SPACECRAFT_DRAFT
            };
            let error = Spacecraft::new(draft)
                .err()
                .expect("A zero inertia diagonal must be rejected");

            assert!(
                error.source().is_some(),
                "A wrapper variant must report the error of the nested field as its source"
            );
        }

        #[test]
        fn a_wrapper_variant_reports_the_mirror_error_of_the_nested_field_as_its_source() {
            let draft = SpacecraftDraft {
                inertia_matrix: diagonal_inertia_draft(0.0, 3.0, 4.0),
                ..VALID_SPACECRAFT_DRAFT
            };
            let error = Spacecraft::new(draft)
                .err()
                .expect("A zero inertia diagonal must be rejected");
            let source = error
                .source()
                .expect("The wrapper variant must report the nested error as its source");

            // The wrapper of the inertia matrix holds the error of the mirror,
            // because the hand-written bridge of the wrapper borrows the error of its mirror
            assert_eq!(
                source.downcast_ref::<InertiaMatrixSerializableValidationError>(),
                Some(&InertiaMatrixSerializableValidationError::OutOfRange {
                    field: InertiaMatrixSerializableField::Xx,
                    range: "]Type::zero(), Type::infinity()[",
                }),
                "The source of the spacecraft error must be the error of the inertia mirror"
            );
        }

        #[test]
        fn a_field_variant_reports_no_source() {
            let error = Spacecraft::new(spacecraft_draft_with_masses(-1.0, 600.0, 300.0))
                .err()
                .expect("A negative mass must be rejected");

            assert!(
                error.source().is_none(),
                "A field variant holds no error, so it must report no source"
            );
        }

        #[test]
        fn spacecraft_new_wraps_the_nested_inertia_error_at_f64_precision() {
            let draft = SpacecraftDraft {
                inertia_matrix: diagonal_inertia_draft(0.0, 3.0, 4.0),
                ..VALID_SPACECRAFT_DRAFT
            };

            assert_eq!(
                Spacecraft::new(draft).err(),
                Some(SpacecraftValidationError::InertiaMatrixValidationError(
                    InertiaMatrixSerializableValidationError::OutOfRange {
                        field: InertiaMatrixSerializableField::Xx,
                        range: "]Type::zero(), Type::infinity()[",
                    }
                )),
                "The spacecraft error must wrap the error of the nested inertia matrix"
            );
        }

        #[test]
        fn spacecraft_new_wraps_the_nested_inertia_error_at_f32_precision() {
            // The shared parameter reaches the nested matrix, so the wrapper variant of the
            // single precision spacecraft holds the error of the single precision mirror
            let draft = SpacecraftDraft {
                inertia_matrix: diagonal_inertia_draft_with_precision(0.0_f32, 3.0_f32, 4.0_f32),
                ..spacecraft_draft_with_area(12.5_f32)
            };

            assert_eq!(
                Spacecraft::new(draft).err(),
                Some(SpacecraftValidationError::InertiaMatrixValidationError(
                    InertiaMatrixSerializableValidationError::OutOfRange {
                        field: InertiaMatrixSerializableField::Xx,
                        range: "]Type::zero(), Type::infinity()[",
                    }
                )),
                "The single precision spacecraft error must wrap the error of its nested matrix"
            );
        }

        #[test]
        fn spacecraft_new_accepts_a_custom_primary_orbited_body() {
            let draft = SpacecraftDraft {
                primary_orbited_body: CelestialBodyKindDraft::Custom(VALID_CELESTIAL_BODY_DRAFT),
                ..VALID_SPACECRAFT_DRAFT
            };
            let spacecraft =
                Spacecraft::new(draft).expect("A valid custom body must build a spacecraft");

            assert_float_eq(
                custom_gravitational_parameter(spacecraft.primary_orbited_body()),
                EARTH_GRAVITATIONAL_PARAMETER,
                "gravitational parameter of the nested custom body",
            );
        }

        #[test]
        fn spacecraft_new_wraps_the_nested_celestial_body_kind_error() {
            let draft = SpacecraftDraft {
                primary_orbited_body: CelestialBodyKindDraft::Custom(CelestialBodyDraft {
                    gravitational_parameter: 0.0,
                }),
                ..VALID_SPACECRAFT_DRAFT
            };

            assert_eq!(
                Spacecraft::new(draft).err(),
                Some(
                    SpacecraftValidationError::PrimaryOrbitedBodyValidationError(
                        CelestialBodyKindValidationError::CustomValidationError(
                            CelestialBodyValidationError::OutOfRange {
                                field: CelestialBodyField::GravitationalParameter,
                                range: "]0.0, +inf[",
                            }
                        )
                    )
                ),
                "The spacecraft error must wrap the error of the nested celestial body kind"
            );
        }

        #[test]
        fn a_wrapper_variant_reports_the_error_of_the_nested_enum_as_its_source() {
            let draft = SpacecraftDraft {
                primary_orbited_body: CelestialBodyKindDraft::Custom(CelestialBodyDraft {
                    gravitational_parameter: 0.0,
                }),
                ..VALID_SPACECRAFT_DRAFT
            };
            let error = Spacecraft::new(draft)
                .err()
                .expect("A custom body with a zero gravitational parameter must be rejected");
            let source = error
                .source()
                .expect("The wrapper variant must report the nested error as its source");

            assert_eq!(
                source.downcast_ref::<CelestialBodyKindValidationError>(),
                Some(&CelestialBodyKindValidationError::CustomValidationError(
                    CelestialBodyValidationError::OutOfRange {
                        field: CelestialBodyField::GravitationalParameter,
                        range: "]0.0, +inf[",
                    }
                )),
                "The source of the spacecraft error must be the error of the celestial body kind"
            );
        }
    }

    /// Conversions between the wrapper, its serde representation and its draft.
    mod conversion {
        use crate::{
            Patch as _, Validate as _,
            tests::{
                assert_float_eq,
                model::{InertiaMatrix, InertiaMatrixSerializable, InertiaMatrixSerializableDraft},
            },
        };

        /// Build an inertia draft where every entry differs.
        /// The distinct entries expose any transposition.
        fn distinct_inertia_draft() -> InertiaMatrixSerializableDraft<f64> {
            InertiaMatrixSerializableDraft {
                xx: 1.0,
                xy: 2.0,
                xz: 3.0,
                yx: 4.0,
                yy: 5.0,
                yz: 6.0,
                zx: 7.0,
                zy: 8.0,
                zz: 9.0,
            }
        }

        #[test]
        fn matrix_mapping_is_not_transposed() {
            let matrix = InertiaMatrix::from_draft_unchecked(distinct_inertia_draft());

            assert_float_eq(matrix.matrix().m11, 1.0, "m11 entry holding xx");
            assert_float_eq(matrix.matrix().m12, 2.0, "m12 entry holding xy");
            assert_float_eq(matrix.matrix().m13, 3.0, "m13 entry holding xz");
            assert_float_eq(matrix.matrix().m21, 4.0, "m21 entry holding yx");
            assert_float_eq(matrix.matrix().m22, 5.0, "m22 entry holding yy");
            assert_float_eq(matrix.matrix().m23, 6.0, "m23 entry holding yz");
            assert_float_eq(matrix.matrix().m31, 7.0, "m31 entry holding zx");
            assert_float_eq(matrix.matrix().m32, 8.0, "m32 entry holding zy");
            assert_float_eq(matrix.matrix().m33, 9.0, "m33 entry holding zz");
        }

        #[test]
        fn inertia_round_trip_is_lossless() {
            let original = InertiaMatrix::from_draft_unchecked(distinct_inertia_draft());
            let restored = InertiaMatrix::from(InertiaMatrixSerializable::from(original));

            assert_float_eq(restored.matrix().m11, 1.0, "m11 entry after the round trip");
            assert_float_eq(restored.matrix().m12, 2.0, "m12 entry after the round trip");
            assert_float_eq(restored.matrix().m13, 3.0, "m13 entry after the round trip");
            assert_float_eq(restored.matrix().m21, 4.0, "m21 entry after the round trip");
            assert_float_eq(restored.matrix().m22, 5.0, "m22 entry after the round trip");
            assert_float_eq(restored.matrix().m23, 6.0, "m23 entry after the round trip");
            assert_float_eq(restored.matrix().m31, 7.0, "m31 entry after the round trip");
            assert_float_eq(restored.matrix().m32, 8.0, "m32 entry after the round trip");
            assert_float_eq(restored.matrix().m33, 9.0, "m33 entry after the round trip");
        }

        #[test]
        fn bridge_to_draft_matches_field_values() {
            let matrix = InertiaMatrix::from_draft_unchecked(distinct_inertia_draft());
            let draft = matrix.to_draft();

            assert_float_eq(draft.xx, 1.0, "xx field of the bridged draft");
            assert_float_eq(draft.xy, 2.0, "xy field of the bridged draft");
            assert_float_eq(draft.xz, 3.0, "xz field of the bridged draft");
            assert_float_eq(draft.yx, 4.0, "yx field of the bridged draft");
            assert_float_eq(draft.yy, 5.0, "yy field of the bridged draft");
            assert_float_eq(draft.yz, 6.0, "yz field of the bridged draft");
            assert_float_eq(draft.zx, 7.0, "zx field of the bridged draft");
            assert_float_eq(draft.zy, 8.0, "zy field of the bridged draft");
            assert_float_eq(draft.zz, 9.0, "zz field of the bridged draft");
        }
    }

    /// Validated setters and draft round trips.
    mod patch {
        use crate::{
            Patch as _, Validate as _,
            tests::{
                EARTH_GRAVITATIONAL_PARAMETER, VALID_CELESTIAL_BODY_DRAFT, VALID_INERTIA_DRAFT,
                VALID_SPACECRAFT_DRAFT, assert_float_eq, custom_gravitational_parameter,
                diagonal_inertia_draft,
                model::{
                    CelestialBodyKind, CelestialBodyKindDraft, InertiaMatrix,
                    InertiaMatrixRealizabilityValidationError, InertiaMatrixSerializable,
                    InertiaMatrixSerializableField, InertiaMatrixSerializableValidationError,
                    ShadowFraction, ShadowFractionDraft, ShadowFractionField,
                    ShadowFractionValidationError, Spacecraft, SpacecraftField,
                    SpacecraftMassSumValidationError, SpacecraftValidationError,
                },
                spacecraft_draft_with_masses,
            },
        };

        #[test]
        fn diagonal_setter_updates_field() {
            let mut matrix = InertiaMatrixSerializable::new(VALID_INERTIA_DRAFT)
                .expect("The valid inertia draft must build an inertia matrix");

            assert_eq!(
                matrix.set_xx(2.5),
                Ok(()),
                "A valid xx update must be accepted"
            );
            assert_float_eq(*matrix.xx(), 2.5, "xx field after the accepted update");
        }

        #[test]
        fn diagonal_setter_rejects_and_leaves_state_unchanged() {
            let mut matrix = InertiaMatrixSerializable::new(VALID_INERTIA_DRAFT)
                .expect("The valid inertia draft must build an inertia matrix");

            assert_eq!(
                matrix.set_xx(0.0),
                Err(InertiaMatrixSerializableValidationError::OutOfRange {
                    field: InertiaMatrixSerializableField::Xx,
                    range: "]Type::zero(), Type::infinity()[",
                }),
                "An xx update to the excluded lower bound must be rejected"
            );

            let expected = VALID_INERTIA_DRAFT;
            assert_float_eq(*matrix.xx(), expected.xx, "xx field after the rejection");
            assert_float_eq(*matrix.xy(), expected.xy, "xy field after the rejection");
            assert_float_eq(*matrix.xz(), expected.xz, "xz field after the rejection");
            assert_float_eq(*matrix.yx(), expected.yx, "yx field after the rejection");
            assert_float_eq(*matrix.yy(), expected.yy, "yy field after the rejection");
            assert_float_eq(*matrix.yz(), expected.yz, "yz field after the rejection");
            assert_float_eq(*matrix.zx(), expected.zx, "zx field after the rejection");
            assert_float_eq(*matrix.zy(), expected.zy, "zy field after the rejection");
            assert_float_eq(*matrix.zz(), expected.zz, "zz field after the rejection");
        }

        #[test]
        fn diagonal_setter_rejects_realizability_violation() {
            let mut matrix = InertiaMatrixSerializable::new(diagonal_inertia_draft(2.0, 2.0, 2.0))
                .expect("The spherical inertia draft must build an inertia matrix");

            assert_eq!(
                matrix.set_zz(5.0),
                Err(
                    InertiaMatrixSerializableValidationError::RealizabilityValidationError(
                        InertiaMatrixRealizabilityValidationError::CovarianceNotPositiveSemiDefinite
                    )
                ),
                "A zz update breaking the triangle inequalities must be rejected"
            );
            assert_float_eq(*matrix.zz(), 2.0, "zz field after the rejection");
        }

        #[test]
        fn shadow_set_value_valid() {
            let mut fraction = ShadowFraction::new(ShadowFractionDraft(0.5))
                .expect("The valid shadow fraction draft must build a shadow fraction");

            assert_eq!(
                fraction.set_value(0.9),
                Ok(()),
                "A valid shadow fraction update must be accepted"
            );
            assert_float_eq(fraction.value(), 0.9, "value after the accepted update");
        }

        #[test]
        fn shadow_set_value_rejects_and_leaves_state_unchanged() {
            let mut fraction = ShadowFraction::new(ShadowFractionDraft(0.5))
                .expect("The valid shadow fraction draft must build a shadow fraction");

            assert_eq!(
                fraction.set_value(2.0),
                Err(ShadowFractionValidationError::OutOfRange {
                    field: ShadowFractionField::Value,
                    range: "[0.0, 1.0]",
                }),
                "A shadow fraction update above the upper bound must be rejected"
            );
            assert_float_eq(fraction.value(), 0.5, "value after the rejection");
        }

        #[test]
        fn spacecraft_set_mass_valid() {
            let mut spacecraft = Spacecraft::new(VALID_SPACECRAFT_DRAFT)
                .expect("The valid spacecraft draft must build a spacecraft");

            assert_eq!(
                spacecraft.set_mass(950.0),
                Ok(()),
                "A total mass still above the sum of the parts must be accepted"
            );
            assert_float_eq(spacecraft.mass(), 950.0, "mass after the accepted update");
        }

        #[test]
        fn spacecraft_set_mass_rejects_mass_sum() {
            let mut spacecraft = Spacecraft::new(VALID_SPACECRAFT_DRAFT)
                .expect("The valid spacecraft draft must build a spacecraft");

            assert_eq!(
                spacecraft.set_mass(100.0),
                Err(SpacecraftValidationError::MassSumValidationError(
                    SpacecraftMassSumValidationError::MassSmallerThanSum
                )),
                "A total mass below the sum of the parts must be rejected"
            );
            assert_float_eq(spacecraft.mass(), 1000.0, "mass after the rejection");
        }

        #[test]
        fn spacecraft_set_bus_mass_reruns_final_validation() {
            let mut spacecraft = Spacecraft::new(VALID_SPACECRAFT_DRAFT)
                .expect("The valid spacecraft draft must build a spacecraft");

            assert_eq!(
                spacecraft.set_bus_mass(800.0),
                Err(SpacecraftValidationError::MassSumValidationError(
                    SpacecraftMassSumValidationError::MassSmallerThanSum
                )),
                "A bus mass pushing the sum above the total mass must be rejected"
            );
            assert_float_eq(spacecraft.bus_mass(), 600.0, "bus mass after the rejection");
        }

        #[test]
        fn spacecraft_set_sail_mass_valid() {
            let mut spacecraft = Spacecraft::new(VALID_SPACECRAFT_DRAFT)
                .expect("The valid spacecraft draft must build a spacecraft");

            assert_eq!(
                spacecraft.set_sail_mass(350.0),
                Ok(()),
                "A sail mass keeping the sum below the total mass must be accepted"
            );
            assert_float_eq(
                spacecraft.sail_mass(),
                350.0,
                "sail mass after the accepted update",
            );
        }

        #[test]
        fn spacecraft_set_area_valid() {
            let mut spacecraft = Spacecraft::new(VALID_SPACECRAFT_DRAFT)
                .expect("The valid spacecraft draft must build a spacecraft");

            assert_eq!(
                spacecraft.set_area(30.0),
                Ok(()),
                "A finite area update must be accepted"
            );
            assert_float_eq(*spacecraft.area(), 30.0, "area after the accepted update");
        }

        #[test]
        fn spacecraft_set_area_rejects_nan_and_leaves_state_unchanged() {
            let mut spacecraft = Spacecraft::new(VALID_SPACECRAFT_DRAFT)
                .expect("The valid spacecraft draft must build a spacecraft");

            assert_eq!(
                spacecraft.set_area(f64::NAN),
                Err(SpacecraftValidationError::NotFinite {
                    field: SpacecraftField::Area,
                }),
                "A not a number area update must be rejected"
            );
            assert_float_eq(*spacecraft.area(), 12.5, "area after the rejection");
        }

        #[test]
        fn spacecraft_set_inertia_matrix_valid() {
            let mut spacecraft = Spacecraft::new(VALID_SPACECRAFT_DRAFT)
                .expect("The valid spacecraft draft must build a spacecraft");
            let new_matrix = InertiaMatrix::from_draft(diagonal_inertia_draft(5.0, 5.0, 5.0))
                .expect("The spherical inertia draft must build an inertia matrix");

            assert_eq!(
                spacecraft.set_inertia_matrix(new_matrix),
                Ok(()),
                "A valid inertia matrix update must be accepted"
            );
            assert_float_eq(
                spacecraft.inertia_matrix().matrix().m11,
                5.0,
                "xx entry after the accepted update",
            );
            assert_float_eq(
                spacecraft.inertia_matrix().to_draft().zz,
                5.0,
                "zz field of the updated matrix draft",
            );
        }

        #[test]
        fn spacecraft_set_sun_shadow_fraction_valid() {
            let mut spacecraft = Spacecraft::new(VALID_SPACECRAFT_DRAFT)
                .expect("The valid spacecraft draft must build a spacecraft");
            let new_fraction = ShadowFraction::new(ShadowFractionDraft(0.1))
                .expect("The valid shadow fraction draft must build a shadow fraction");

            assert_eq!(
                spacecraft.set_sun_shadow_fraction(new_fraction),
                Ok(()),
                "A valid shadow fraction update must be accepted"
            );
            assert_float_eq(
                spacecraft.sun_shadow_fraction().value(),
                0.1,
                "shadow fraction after the accepted update",
            );
        }

        #[test]
        fn spacecraft_set_primary_orbited_body_valid() {
            let mut spacecraft = Spacecraft::new(VALID_SPACECRAFT_DRAFT)
                .expect("The valid spacecraft draft must build a spacecraft");
            let custom_body =
                CelestialBodyKind::new(CelestialBodyKindDraft::Custom(VALID_CELESTIAL_BODY_DRAFT))
                    .expect("The valid celestial body draft must build a celestial body kind");

            assert_eq!(
                spacecraft.set_primary_orbited_body(custom_body),
                Ok(()),
                "A valid custom body update must be accepted"
            );
            assert_float_eq(
                custom_gravitational_parameter(spacecraft.primary_orbited_body()),
                EARTH_GRAVITATIONAL_PARAMETER,
                "gravitational parameter after the accepted update",
            );

            assert_eq!(
                spacecraft.set_primary_orbited_body(CelestialBodyKind::Sun),
                Ok(()),
                "A unit variant update must be accepted"
            );
            assert!(
                matches!(spacecraft.primary_orbited_body(), CelestialBodyKind::Sun),
                "The nested field setter must store the new variant"
            );
        }

        #[test]
        fn spacecraft_set_primary_orbited_body_runs_the_final_validation() {
            // The unchecked constructor builds a spacecraft that already breaks the mass sum,
            // so the patch of the nested body must report the final validation error
            let mut invalid_spacecraft =
                Spacecraft::from_draft_unchecked(spacecraft_draft_with_masses(100.0, 600.0, 300.0));

            assert_eq!(
                invalid_spacecraft.set_primary_orbited_body(CelestialBodyKind::Sun),
                Err(SpacecraftValidationError::MassSumValidationError(
                    SpacecraftMassSumValidationError::MassSmallerThanSum
                )),
                "The nested field setter must run the final validation of the type"
            );
        }

        #[test]
        fn to_draft_from_draft_round_trip() {
            let matrix = InertiaMatrixSerializable::new(VALID_INERTIA_DRAFT)
                .expect("The valid inertia draft must build an inertia matrix");
            let restored_matrix = InertiaMatrixSerializable::from_draft(matrix.to_draft())
                .expect("The draft of a valid inertia matrix must validate again");
            assert_float_eq(
                *restored_matrix.xx(),
                2.0,
                "xx field of the round tripped matrix",
            );

            let fraction = ShadowFraction::new(ShadowFractionDraft(0.25))
                .expect("The valid shadow fraction draft must build a shadow fraction");
            let restored_fraction = ShadowFraction::from_draft(fraction.to_draft())
                .expect("The draft of a valid shadow fraction must validate again");
            assert_float_eq(
                restored_fraction.value(),
                0.25,
                "value of the round tripped shadow fraction",
            );

            let spacecraft = Spacecraft::new(VALID_SPACECRAFT_DRAFT)
                .expect("The valid spacecraft draft must build a spacecraft");
            let restored_spacecraft = Spacecraft::from_draft(spacecraft.to_draft())
                .expect("The draft of a valid spacecraft must validate again");
            assert_float_eq(
                restored_spacecraft.mass(),
                1000.0,
                "mass of the round tripped spacecraft",
            );
        }
    }

    /// Serde wire format and deserialization.
    mod serde_integration {
        use crate::tests::{
            EARTH_GRAVITATIONAL_PARAMETER, VALID_CELESTIAL_BODY_DRAFT, VALID_SPACECRAFT_DRAFT,
            assert_float_eq, custom_gravitational_parameter,
            model::{
                CelestialBody, CelestialBodyDraft, CelestialBodyKind, CelestialBodyKindDraft,
                InertiaMatrix, ShadowFraction, Spacecraft, SpacecraftDraft,
            },
            spacecraft_draft_with_area,
        };

        /// Build the JSON document of a spacecraft with the given masses and inertia `inertia_xx`.
        /// Every other entry matches the standard valid spacecraft draft.
        fn spacecraft_json_value(
            mass: f64,
            bus_mass: f64,
            sail_mass: f64,
            inertia_xx: f64,
        ) -> serde_json::Value {
            serde_json::json!({
                "mass": mass,
                "bus_mass": bus_mass,
                "sail_mass": sail_mass,
                "area": 12.5,
                "inertia_matrix": {
                    "xx": inertia_xx,
                    "xy": 0.0,
                    "xz": 0.0,
                    "yx": 0.0,
                    "yy": 3.0,
                    "yz": 0.0,
                    "zx": 0.0,
                    "zy": 0.0,
                    "zz": 4.0
                },
                "sun_shadow_fraction": 0.5,
                "primary_orbited_body": "Earth"
            })
        }

        /// Build the JSON document that matches the standard valid spacecraft draft.
        fn valid_spacecraft_json_value() -> serde_json::Value {
            spacecraft_json_value(1000.0, 600.0, 300.0, 2.0)
        }

        /// Return the message of the failed deserialization `result`.
        /// The `described_as` argument names the document that the deserialization must reject.
        fn deserialization_error_message<Deserialized>(
            result: Result<Deserialized, serde_json::Error>,
            described_as: &str,
        ) -> String {
            match result {
                Ok(_) => panic!("The {described_as} must be rejected during deserialization"),
                Err(error) => error.to_string(),
            }
        }

        #[test]
        fn shadow_deserialize_valid() {
            let fraction = serde_json::from_str::<ShadowFraction>("0.5")
                .expect("A bare number inside the range must deserialize");

            assert_float_eq(fraction.value(), 0.5, "deserialized shadow fraction value");
        }

        #[test]
        fn shadow_deserialize_out_of_range_rejected() {
            let message = deserialization_error_message(
                serde_json::from_str::<ShadowFraction>("1.5"),
                "shadow fraction above the upper bound",
            );

            assert!(
                message.contains("range"),
                "The rejection message must mention the range but was {message}"
            );
        }

        #[test]
        fn spacecraft_deserialize_valid() {
            let spacecraft =
                serde_json::from_str::<Spacecraft>(&valid_spacecraft_json_value().to_string())
                    .expect("The valid spacecraft document must deserialize");

            assert_float_eq(spacecraft.mass(), 1000.0, "deserialized mass");
            assert_float_eq(spacecraft.bus_mass(), 600.0, "deserialized bus mass");
            assert_float_eq(spacecraft.sail_mass(), 300.0, "deserialized sail mass");
            assert_float_eq(
                spacecraft.inertia_matrix().matrix().m11,
                2.0,
                "deserialized xx entry",
            );
            assert_float_eq(
                spacecraft.sun_shadow_fraction().value(),
                0.5,
                "deserialized shadow fraction",
            );
            assert!(
                matches!(spacecraft.primary_orbited_body(), CelestialBodyKind::Earth),
                "The deserialized primary orbited body must be Earth"
            );
        }

        #[test]
        fn spacecraft_serialize_deserialize_round_trip_at_f64_precision() {
            let spacecraft = Spacecraft::new(VALID_SPACECRAFT_DRAFT)
                .expect("The valid spacecraft draft must build a spacecraft");
            let document = serde_json::to_string(&spacecraft)
                .expect("A valid spacecraft must serialize to a document");
            let restored = serde_json::from_str::<Spacecraft>(&document)
                .expect("The serialized document must deserialize back");

            assert_float_eq(restored.mass(), 1000.0, "round tripped mass");
            assert_float_eq(restored.bus_mass(), 600.0, "round tripped bus mass");
            assert_float_eq(restored.sail_mass(), 300.0, "round tripped sail mass");
            assert_float_eq(
                restored.inertia_matrix().matrix().m22,
                3.0,
                "round tripped yy entry",
            );
            assert_float_eq(
                restored.sun_shadow_fraction().value(),
                0.5,
                "round tripped shadow fraction",
            );
        }

        #[test]
        fn spacecraft_serialize_deserialize_round_trip_at_f32_precision() {
            let spacecraft = Spacecraft::new(spacecraft_draft_with_area(12.5_f32))
                .expect("The valid single precision spacecraft draft must build a spacecraft");
            let document = serde_json::to_string(&spacecraft)
                .expect("A valid single precision spacecraft must serialize to a document");
            let restored = serde_json::from_str::<Spacecraft<f32>>(&document)
                .expect("The serialized document must deserialize back");

            // The helper of the double precision cannot compare a single precision value,
            // so the checks read the bit patterns of the values directly
            assert_eq!(
                restored.area().to_bits(),
                12.5_f32.to_bits(),
                "The round tripped area must be exactly 12.5 but was {}",
                restored.area()
            );
            assert_eq!(
                restored.inertia_matrix().matrix().m22.to_bits(),
                3.0_f32.to_bits(),
                "The round tripped yy entry must be exactly 3.0 but was {}",
                restored.inertia_matrix().matrix().m22
            );
        }

        #[test]
        fn spacecraft_deserialize_rejects_invalid_nested_inertia_at_f64_precision() {
            let document = spacecraft_json_value(1000.0, 600.0, 300.0, 0.0).to_string();
            let message = deserialization_error_message(
                serde_json::from_str::<Spacecraft>(&document),
                "spacecraft document holding a zero inertia diagonal",
            );

            assert!(
                message.contains("range"),
                "The rejection message must mention the range but was {message}"
            );
        }

        #[test]
        fn spacecraft_deserialize_rejects_invalid_nested_inertia_at_f32_precision() {
            let document = spacecraft_json_value(1000.0, 600.0, 300.0, 0.0).to_string();
            let message = deserialization_error_message(
                serde_json::from_str::<Spacecraft<f32>>(&document),
                "single precision spacecraft document holding a zero inertia diagonal",
            );

            assert!(
                message.contains("range"),
                "The rejection message must mention the range but was {message}"
            );
        }

        #[test]
        fn spacecraft_deserialize_rejects_mass_sum_violation() {
            let document = spacecraft_json_value(100.0, 600.0, 300.0, 2.0).to_string();
            let message = deserialization_error_message(
                serde_json::from_str::<Spacecraft>(&document),
                "spacecraft document holding a total mass below the sum of its parts",
            );

            assert!(
                message.contains("mass"),
                "The rejection message must mention the mass but was {message}"
            );
        }

        #[test]
        fn spacecraft_wire_format_stable() {
            let spacecraft = Spacecraft::new(VALID_SPACECRAFT_DRAFT)
                .expect("The valid spacecraft draft must build a spacecraft");

            assert_eq!(
                serde_json::to_value(&spacecraft)
                    .expect("A valid spacecraft must serialize to a value"),
                valid_spacecraft_json_value(),
                "The spacecraft wire format must stay stable"
            );
        }

        #[test]
        fn celestial_body_kind_wire_tags() {
            assert_eq!(
                serde_json::to_string(&CelestialBodyKind::Sun)
                    .expect("The Sun variant must serialize"),
                "\"Sun\"",
                "The Sun variant must serialize to its own name"
            );
            assert_eq!(
                serde_json::to_string(&CelestialBodyKind::Earth)
                    .expect("The Earth variant must serialize"),
                "\"Earth\"",
                "The Earth variant must serialize to its own name"
            );
            let custom_body = CelestialBodyKind::Custom(
                CelestialBody::new(CelestialBodyDraft {
                    gravitational_parameter: 1.0,
                })
                .expect("A gravitational parameter of 1.0 must build a celestial body"),
            );
            assert_eq!(
                serde_json::to_string(&custom_body).expect("The Custom variant must serialize"),
                "{\"Custom\":{\"gravitational_parameter\":1.0}}",
                "The Custom variant must serialize to its own name and to its payload"
            );
        }

        #[test]
        fn spacecraft_deserialize_rejects_an_invalid_custom_body() {
            // The draft of the spacecraft carries the wire format of the spacecraft,
            // so it builds the document that the validation must reject
            let document = serde_json::to_string(&SpacecraftDraft {
                primary_orbited_body: CelestialBodyKindDraft::Custom(CelestialBodyDraft {
                    gravitational_parameter: 0.0,
                }),
                ..VALID_SPACECRAFT_DRAFT
            })
            .expect("A spacecraft draft must serialize to a document");
            let message = deserialization_error_message(
                serde_json::from_str::<Spacecraft>(&document),
                "spacecraft document holding a custom body with a zero gravitational parameter",
            );

            assert!(
                message.contains("range"),
                "The rejection message must mention the range but was {message}"
            );
        }

        #[test]
        fn spacecraft_custom_body_serialize_deserialize_round_trip() {
            let spacecraft = Spacecraft::new(SpacecraftDraft {
                primary_orbited_body: CelestialBodyKindDraft::Custom(VALID_CELESTIAL_BODY_DRAFT),
                ..VALID_SPACECRAFT_DRAFT
            })
            .expect("A valid custom body must build a spacecraft");
            let document = serde_json::to_string(&spacecraft)
                .expect("A valid spacecraft must serialize to a document");
            let restored = serde_json::from_str::<Spacecraft>(&document)
                .expect("The serialized document must deserialize back");

            assert_float_eq(
                custom_gravitational_parameter(restored.primary_orbited_body()),
                EARTH_GRAVITATIONAL_PARAMETER,
                "round tripped gravitational parameter of the custom body",
            );
        }

        #[test]
        fn inertia_matrix_deserialize_runs_the_validation_of_its_serde_representation() {
            // The wrapper deserializes through its serde representation,
            // so the validation of the representation also guards the wrapper.
            let document = serde_json::json!({
                "xx": 2.0,
                "xy": 5.0,
                "xz": 0.0,
                "yx": 0.0,
                "yy": 3.0,
                "yz": 0.0,
                "zx": 0.0,
                "zy": 0.0,
                "zz": 4.0
            })
            .to_string();
            let message = deserialization_error_message(
                serde_json::from_str::<InertiaMatrix<f64>>(&document),
                "asymmetric inertia document",
            );

            assert!(
                message.contains("symmetry"),
                "The rejection message must mention the symmetry but was {message}"
            );
        }
    }
}
