//! Validated spacecraft model.

use core::ops::Bound;

use nalgebra::{ComplexField, Matrix3};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use valide::{Patch, Validate};

/// List of supported celestial body kinds.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum CelestialBodyKind {
    /// Sun.
    Sun,
    /// Earth.
    Earth,
    // TODO: Add a tuple variant
}

/// Inertia matrix of a body.
#[repr(transparent)]
#[derive(Clone, PartialEq)]
pub struct InertiaMatrix(Matrix3<f64>);

impl InertiaMatrix {
    /// Retrieve the wrapped matrix.
    pub fn matrix(&self) -> &Matrix3<f64> {
        &self.0
    }
}

impl Serialize for InertiaMatrix {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        InertiaMatrixSerializable::from(self.clone()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for InertiaMatrix {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = InertiaMatrixSerializable::deserialize(deserializer)?;
        Ok(Self::from(value))
    }
}

impl From<InertiaMatrixSerializable> for InertiaMatrix {
    fn from(value: InertiaMatrixSerializable) -> Self {
        Self(Matrix3::new(
            value.xx, value.xy, value.xz, value.yx, value.yy, value.yz, value.zx, value.zy,
            value.zz,
        ))
    }
}

impl From<InertiaMatrix> for InertiaMatrixSerializable {
    fn from(value: InertiaMatrix) -> Self {
        Self {
            xx: value.0.m11,
            xy: value.0.m12,
            xz: value.0.m13,
            yx: value.0.m21,
            yy: value.0.m22,
            yz: value.0.m23,
            zx: value.0.m31,
            zy: value.0.m32,
            zz: value.0.m33,
        }
    }
}

// Hand-written bridge declaring the draft type and conversion path of the wrapper,
// written once so the generated code never has to know them
impl Validate for InertiaMatrix {
    type Draft = InertiaMatrixSerializableDraft;
    type Error = InertiaMatrixSerializableValidationError;

    fn validate(draft: &Self::Draft) -> Result<(), Self::Error> {
        draft.validate()
    }

    fn from_draft_unchecked(draft: Self::Draft) -> Self {
        InertiaMatrixSerializable::from_draft_unchecked(draft).into()
    }
}

impl Patch for InertiaMatrix {
    fn to_draft(&self) -> Self::Draft {
        InertiaMatrixSerializable::from(self.clone()).into()
    }
}

/// Serde representation of an [`InertiaMatrix`].
#[derive(Clone, Serialize, Deserialize, valide_derive::Validate, valide_derive::Patch)]
#[serde(try_from = "InertiaMatrixSerializableDraft")]
#[final_validation(validate_realizability, error = InertiaMatrixRealizabilityValidationError)]
pub struct InertiaMatrixSerializable {
    /// Ixx.
    #[validate(range(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)))]
    xx: f64,
    /// Ixy.
    #[validate(finite)]
    xy: f64,
    /// Ixz.
    #[validate(finite)]
    xz: f64,
    /// Iyx.
    #[validate(finite)]
    yx: f64,
    /// Iyy.
    #[validate(range(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)))]
    yy: f64,
    /// Iyz.
    #[validate(finite)]
    yz: f64,
    /// Izx.
    #[validate(finite)]
    zx: f64,
    /// Izy.
    #[validate(finite)]
    zy: f64,
    /// Izz.
    #[validate(range(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)))]
    zz: f64,
}

/// Error type of the [`InertiaMatrixSerializable::validate_realizability`] validation.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum InertiaMatrixRealizabilityValidationError {
    /// At least one off-diagonal pair differs by more than [`SYMMETRY_TOLERANCE`].
    #[error("The inertia matrix off-diagonal entries do not match within the symmetry tolerance")]
    NotSymmetric,
    /// The trace of the input matrix is negative, so no mass distribution can produce it.
    #[error("The inertia matrix trace must be non-negative")]
    NegativeTrace,
    /// The derived mass covariance matrix is not positive semi-definite.
    /// The principal moments of inertia do not respect the triangle inequalities.
    #[error("The mass covariance matrix must be positive semi-definite")]
    CovarianceNotPositiveSemiDefinite,
}

#[expect(
    clippy::multiple_inherent_impl,
    reason = "derives generate some part of the struct"
)]
impl InertiaMatrixSerializable {
    /// Absolute tolerance on the off-diagonal mismatch of the symmetry check.
    pub const SYMMETRY_TOLERANCE: f64 = 1e-9;

    /// Absolute tolerance of the physical realizability check.
    /// The tolerance uses the unit of the compared quantity.
    pub const REALIZABILITY_TOLERANCE: f64 = 1e-9;

    /// Check that the matrix of the given `draft` is symmetric within
    /// [`Self::SYMMETRY_TOLERANCE`].
    /// Check that it is a physically realizable mass distribution within
    /// [`Self::REALIZABILITY_TOLERANCE`].
    /// The entries must be finite, which the field validators enforce.
    pub fn validate_realizability(
        draft: &InertiaMatrixSerializableDraft,
    ) -> Result<(), InertiaMatrixRealizabilityValidationError> {
        // Check symmetry
        if !((draft.xy - draft.yx).abs() <= Self::SYMMETRY_TOLERANCE
            && (draft.xz - draft.zx).abs() <= Self::SYMMETRY_TOLERANCE
            && (draft.yz - draft.zy).abs() <= Self::SYMMETRY_TOLERANCE)
        {
            return Err(InertiaMatrixRealizabilityValidationError::NotSymmetric);
        }

        // Check realizability
        let half_trace = 0.5 * (draft.xx + draft.yy + draft.zz);
        if half_trace < -Self::REALIZABILITY_TOLERANCE {
            return Err(InertiaMatrixRealizabilityValidationError::NegativeTrace);
        }

        // Mass covariance matrix: sigma = half_trace * identity - inertia. Symmetry
        // was checked above, so only the upper triangle entries are used.
        let s00 = half_trace - draft.xx;
        let s11 = half_trace - draft.yy;
        let s22 = half_trace - draft.zz;
        let s01 = -draft.xy;
        let s02 = -draft.xz;
        let s12 = -draft.yz;

        // Sylvester's criterion for positive semi-definiteness: ALL principal minors
        // of sigma must be non-negative (leading minors alone only prove definiteness).
        // Order 1: diagonal entries.
        let diagonal_ok = s00 >= -Self::REALIZABILITY_TOLERANCE
            && s11 >= -Self::REALIZABILITY_TOLERANCE
            && s22 >= -Self::REALIZABILITY_TOLERANCE;

        // The qualified ComplexField calls make the fused multiply add follow the selected
        // math feature, the inherent f64 method would always resolve to the native one
        // Order 2: the three 2x2 principal minors.
        let minor_2_ok = ComplexField::mul_add(s01, -s01, s00 * s11)
            >= -Self::REALIZABILITY_TOLERANCE
            && ComplexField::mul_add(s02, -s02, s00 * s22) >= -Self::REALIZABILITY_TOLERANCE
            && ComplexField::mul_add(s12, -s12, s11 * s22) >= -Self::REALIZABILITY_TOLERANCE;

        // Order 3: the determinant.
        let determinant = ComplexField::mul_add(
            s02,
            ComplexField::mul_add(s01, s12, -(s11 * s02)),
            ComplexField::mul_add(
                s01,
                -ComplexField::mul_add(s12, -s02, s01 * s22),
                s00 * ComplexField::mul_add(s12, -s12, s11 * s22),
            ),
        );
        let determinant_ok = determinant >= -Self::REALIZABILITY_TOLERANCE;

        if !diagonal_ok || !minor_2_ok || !determinant_ok {
            return Err(
                InertiaMatrixRealizabilityValidationError::CovarianceNotPositiveSemiDefinite,
            );
        }

        Ok(())
    }
}
/// Fraction of the sunlight that reaches a spacecraft, bounded to [0.0, 1.0].
///
/// A value of 1.0 represents full sunlight and 0.0 represents a full eclipse.
#[repr(transparent)]
#[derive(
    Clone, PartialEq, Serialize, Deserialize, valide_derive::Validate, valide_derive::Patch,
)]
#[serde(try_from = "ShadowFractionDraft")]
pub struct ShadowFraction(#[validate(range(0.0..=1.0))] f64);

/// Reference physical properties of a spacecraft that the dynamical simulations use.
#[derive(
    Clone, PartialEq, Serialize, Deserialize, valide_derive::Validate, valide_derive::Patch,
)]
#[serde(try_from = "SpacecraftDraft<Area>")]
#[final_validation(validate_mass_sum, error = SpacecraftMassSumValidationError)]
pub struct Spacecraft<Area: ComplexField = f64> {
    /// Total spacecraft mass in kilograms (kg).
    #[validate(range(0.0..f64::INFINITY))]
    mass: f64,
    /// Mass of the spacecraft bus in kilograms (kg).
    #[validate(range(0.0..=30_000.0))]
    bus_mass: f64,
    /// Mass of the spacecraft sail in kilograms (kg).
    #[validate(range(0.0..=10_000.0))]
    sail_mass: f64,
    /// Cross-sectional area of the spacecraft in square meters (m²).
    #[validate(finite)]
    area: Area,
    /// Moment of inertia matrix in the body frame (kg·m²).
    #[validate(nested)]
    inertia_matrix: InertiaMatrix,
    /// Fraction of sunlight reaching the spacecraft.
    #[validate(nested)]
    sun_shadow_fraction: ShadowFraction,
    /// Celestial body that this spacecraft primarily orbits.
    // A skip field must not be read by any final validation function
    #[validate(skip)]
    primary_orbited_body: CelestialBodyKind,
}

/// Error type of the [`Spacecraft::validate_mass_sum`] validation.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum SpacecraftMassSumValidationError {
    /// The spacecraft total mass is smaller than the sum of the bus and sail mass
    /// by more than [`Spacecraft::MASS_SUM_TOLERANCE`].
    #[error(
        "The spacecraft total mass must be greater or equal to the sum of the bus and sail mass within the mass sum tolerance"
    )]
    MassSmallerThanSum,
}

#[expect(
    clippy::multiple_inherent_impl,
    reason = "derives generate some part of the struct"
)]
impl<Area: ComplexField> Spacecraft<Area> {
    /// Absolute tolerance for the mass sum check, in kilograms (kg).
    pub const MASS_SUM_TOLERANCE: f64 = 1e-9;

    /// Check that the `mass` of the given `draft` is greater than or equal to the sum of
    /// its `bus_mass` and `sail_mass`, within [`Self::MASS_SUM_TOLERANCE`].
    pub fn validate_mass_sum(
        draft: &SpacecraftDraft<Area>,
    ) -> Result<(), SpacecraftMassSumValidationError> {
        if draft.mass < draft.bus_mass + draft.sail_mass - Self::MASS_SUM_TOLERANCE {
            return Err(SpacecraftMassSumValidationError::MassSmallerThanSum);
        }

        Ok(())
    }
}
