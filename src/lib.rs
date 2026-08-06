//! A crate for validating types.

use core::ops::{Bound, RangeBounds as _};

use nalgebra::Matrix3;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// List of supported celestial body kinds.
#[derive(Clone, Serialize, Deserialize)]
pub enum CelestialBodyKind {
    /// Sun.
    Sun,
    /// Earth.
    Earth,
    // TODO: Add a tuple variant
}

/* --------- WRITTEN ------------ */

/// Inertial Matrix of a body.
#[repr(transparent)]
#[derive(Clone)]
pub struct InertiaMatrix(Matrix3<f64>);

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

/// Serde representation of a [`InertiaMatrix`].
#[derive(Clone, Serialize, Deserialize)]
// #[serde(try_from = "InertiaMatrixSerializableDraft")]
// #[derive(Validate, Patchable)]
// #[final_validation(validate_realizability)]
pub struct InertiaMatrixSerializable {
    /// Ixx.
    // #[validate(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)))]
    xx: f64,
    /// Ixy.
    xy: f64,
    /// Ixz.
    xz: f64,
    /// Iyx.
    yx: f64,
    /// Iyy.
    // #[validate(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)))]
    yy: f64,
    /// Iyz.
    yz: f64,
    /// Izx.
    zx: f64,
    /// Izy.
    zy: f64,
    /// Izz.
    // #[validate(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)))]
    zz: f64,
}

/// Error type for [`InertiaMatrixSerializable::validate_realizability`] validation failures.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum SpacecraftRealizabilityValidationError {
    /// At least one off-diagonal pair differs by more than [`SYMMETRY_TOLERANCE`].
    #[error("The inertia matrix off-diagonal entries do not match within the symmetry tolerance")]
    NotSymmetric,
    /// The trace of the input matrix is negative, so no mass distribution can produce it.
    #[error("The inertia matrix trace must be non-negative")]
    NegativeTrace,
    /// The derived mass covariance matrix is not positive semi-definite,
    /// violating the triangle inequalities on the principal moments of inertia.
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

    /// Absolute tolerance for the physical realizability check, in the unit of the compared quantity.
    pub const REALIZABILITY_TOLERANCE: f64 = 1e-9;

    /// Validation function to verify that the given spacecraft `mass`
    /// is smaller than the sum of the given `bus_mass` and `sail_mass`.
    pub fn validate_realizability(
        draft: &InertiaMatrixSerializableDraft,
    ) -> Result<(), SpacecraftRealizabilityValidationError> {
        // Check symmetry
        if !((draft.xy - draft.yx).abs() <= Self::SYMMETRY_TOLERANCE
            && (draft.xz - draft.zx).abs() <= Self::SYMMETRY_TOLERANCE
            && (draft.yz - draft.zy).abs() <= Self::SYMMETRY_TOLERANCE)
        {
            return Err(SpacecraftRealizabilityValidationError::NotSymmetric);
        }

        // Check realizability
        let half_trace = 0.5 * (draft.xx + draft.yy + draft.zz);
        if half_trace < -Self::REALIZABILITY_TOLERANCE {
            return Err(SpacecraftRealizabilityValidationError::NegativeTrace);
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

        // Order 2: the three 2x2 principal minors.
        let minor_2_ok = s01.mul_add(-s01, s00 * s11) >= -Self::REALIZABILITY_TOLERANCE
            && s02.mul_add(-s02, s00 * s22) >= -Self::REALIZABILITY_TOLERANCE
            && s12.mul_add(-s12, s11 * s22) >= -Self::REALIZABILITY_TOLERANCE;

        // Order 3: the determinant.
        let determinant = s02.mul_add(
            s01.mul_add(s12, -(s11 * s02)),
            s01.mul_add(
                -s12.mul_add(-s02, s01 * s22),
                s00 * s12.mul_add(-s12, s11 * s22),
            ),
        );
        let determinant_ok = determinant >= -Self::REALIZABILITY_TOLERANCE;

        if !diagonal_ok || !minor_2_ok || !determinant_ok {
            return Err(SpacecraftRealizabilityValidationError::CovarianceNotPositiveSemiDefinite);
        }

        Ok(())
    }
}

/* --------- GENERATED ------------ */

/// Error type for [`InertiaMatrixSerializable`] validation failures.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum InertiaMatrixSerializableValidationError {
    /// The value is outside the valid range `[0.0, 1.0]`.
    #[error("The value must be within the range [0.0, 1.0]")]
    OutOfRange,
    /// The validate_realizability validation failed.
    #[error("{0}")]
    SpacecraftRealizabilityValidationError(SpacecraftRealizabilityValidationError),
}

/// Draft construction of a [`InertiaMatrixSerializable`].
#[derive(Serialize, Deserialize)]
pub struct InertiaMatrixSerializableDraft {
    /// Ixx.
    xx: f64,
    /// Ixy.
    xy: f64,
    /// Ixz.
    xz: f64,
    /// Iyx.
    yx: f64,
    /// Iyy.
    yy: f64,
    /// Iyz.
    yz: f64,
    /// Izx.
    zx: f64,
    /// Izy.
    zy: f64,
    /// Izz.
    zz: f64,
}

impl InertiaMatrixSerializableDraft {
    /// Validate all the draft with fail fast policy where the first error found is directly returned.
    pub fn validate(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        self.validate_xx()?;
        self.validate_yy()?;
        self.validate_zz()?;

        InertiaMatrixSerializable::validate_realizability(self).map_err(
            InertiaMatrixSerializableValidationError::SpacecraftRealizabilityValidationError,
        )?;

        Ok(())
    }

    /// Validate the `xx` field.
    pub fn validate_xx(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)).contains(&self.xx) {
            return Err(InertiaMatrixSerializableValidationError::OutOfRange);
        }

        Ok(())
    }

    /// Validate the `yy` field.
    pub fn validate_yy(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)).contains(&self.yy) {
            return Err(InertiaMatrixSerializableValidationError::OutOfRange);
        }

        Ok(())
    }

    /// Validate the `zz` field.
    pub fn validate_zz(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)).contains(&self.zz) {
            return Err(InertiaMatrixSerializableValidationError::OutOfRange);
        }

        Ok(())
    }
}

impl From<InertiaMatrixSerializable> for InertiaMatrixSerializableDraft {
    fn from(value: InertiaMatrixSerializable) -> Self {
        Self {
            xx: value.xx,
            xy: value.xy,
            xz: value.xz,
            yx: value.yx,
            yy: value.yy,
            yz: value.yz,
            zx: value.zx,
            zy: value.zy,
            zz: value.zz,
        }
    }
}

impl InertiaMatrixSerializable {
    /// Create a new [`InertiaMatrixSerializable`] from the given `draft`.
    /// Return the first error found during `draft` validation.
    pub fn new(
        draft: InertiaMatrixSerializableDraft,
    ) -> Result<Self, InertiaMatrixSerializableValidationError> {
        Self::try_from(draft)
    }

    /// Retrieve the `xx` field.
    pub fn xx(&self) -> f64 {
        self.xx
    }

    /// Retrieve the `xy` field.
    pub fn xy(&self) -> f64 {
        self.xy
    }

    /// Retrieve the `xz` field.
    pub fn xz(&self) -> f64 {
        self.xz
    }

    /// Retrieve the `yx` field.
    pub fn yx(&self) -> f64 {
        self.yx
    }

    /// Retrieve the `yy` field.
    pub fn yy(&self) -> f64 {
        self.yy
    }

    /// Retrieve the `yz` field.
    pub fn yz(&self) -> f64 {
        self.yz
    }

    /// Retrieve the `zx` field.
    pub fn zx(&self) -> f64 {
        self.zx
    }

    /// Retrieve the `zy` field.
    pub fn zy(&self) -> f64 {
        self.zy
    }

    /// Retrieve the `zz` field.
    pub fn zz(&self) -> f64 {
        self.zz
    }

    /// Set the given `new_xx`.
    /// Return an error if the `new_xx` cannot be validated.
    pub fn set_xx(&mut self, new_xx: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.xx = new_xx;
        let _: () = tmp_draft.validate_xx()?;

        Self::validate_realizability(&tmp_draft).map_err(
            InertiaMatrixSerializableValidationError::SpacecraftRealizabilityValidationError,
        )?;

        self.xx = new_xx;

        Ok(())
    }

    /// Set the given `new_xy`.
    /// Return an error if the `new_xy` cannot be validated.
    pub fn set_xy(&mut self, new_xy: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.xy = new_xy;

        Self::validate_realizability(&tmp_draft).map_err(
            InertiaMatrixSerializableValidationError::SpacecraftRealizabilityValidationError,
        )?;

        self.xy = new_xy;

        Ok(())
    }

    /// Set the given `new_xz`.
    /// Return an error if the `new_xz` cannot be validated.
    pub fn set_xz(&mut self, new_xz: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.xz = new_xz;

        Self::validate_realizability(&tmp_draft).map_err(
            InertiaMatrixSerializableValidationError::SpacecraftRealizabilityValidationError,
        )?;

        self.xz = new_xz;

        Ok(())
    }

    /// Set the given `new_yx`.
    /// Return an error if the `new_yx` cannot be validated.
    pub fn set_yx(&mut self, new_yx: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.yx = new_yx;

        Self::validate_realizability(&tmp_draft).map_err(
            InertiaMatrixSerializableValidationError::SpacecraftRealizabilityValidationError,
        )?;

        self.yx = new_yx;

        Ok(())
    }

    /// Set the given `new_yy`.
    /// Return an error if the `new_yy` cannot be validated.
    pub fn set_yy(&mut self, new_yy: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.yy = new_yy;
        let _: () = tmp_draft.validate_yy()?;

        Self::validate_realizability(&tmp_draft).map_err(
            InertiaMatrixSerializableValidationError::SpacecraftRealizabilityValidationError,
        )?;

        self.yy = new_yy;

        Ok(())
    }

    /// Set the given `new_yz`.
    /// Return an error if the `new_yz` cannot be validated.
    pub fn set_yz(&mut self, new_yz: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.yz = new_yz;

        Self::validate_realizability(&tmp_draft).map_err(
            InertiaMatrixSerializableValidationError::SpacecraftRealizabilityValidationError,
        )?;

        self.yz = new_yz;

        Ok(())
    }

    /// Set the given `new_zx`.
    /// Return an error if the `new_zx` cannot be validated.
    pub fn set_zx(&mut self, new_zx: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.zx = new_zx;

        Self::validate_realizability(&tmp_draft).map_err(
            InertiaMatrixSerializableValidationError::SpacecraftRealizabilityValidationError,
        )?;

        self.zx = new_zx;

        Ok(())
    }

    /// Set the given `new_zy`.
    /// Return an error if the `new_zy` cannot be validated.
    pub fn set_zy(&mut self, new_zy: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.zy = new_zy;

        Self::validate_realizability(&tmp_draft).map_err(
            InertiaMatrixSerializableValidationError::SpacecraftRealizabilityValidationError,
        )?;

        self.zy = new_zy;

        Ok(())
    }

    /// Set the given `new_zz`.
    /// Return an error if the `new_zz` cannot be validated.
    pub fn set_zz(&mut self, new_zz: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.zz = new_zz;
        let _: () = tmp_draft.validate_zz()?;

        Self::validate_realizability(&tmp_draft).map_err(
            InertiaMatrixSerializableValidationError::SpacecraftRealizabilityValidationError,
        )?;

        self.zz = new_zz;

        Ok(())
    }
}

impl TryFrom<InertiaMatrixSerializableDraft> for InertiaMatrixSerializable {
    type Error = InertiaMatrixSerializableValidationError;

    fn try_from(value: InertiaMatrixSerializableDraft) -> Result<Self, Self::Error> {
        value.validate()?;

        Ok(Self {
            xx: value.xx,
            xy: value.xy,
            xz: value.xz,
            yx: value.yx,
            yy: value.yy,
            yz: value.yz,
            zx: value.zx,
            zy: value.zy,
            zz: value.zz,
        })
    }
}

/* --------- WRITTEN ------------ */

/// Fraction of sunlight reaching a spacecraft, bounded to [0.0, 1.0].
///
/// A value of 1.0 represents full sunlight and 0.0 represents full eclipse.
#[repr(transparent)]
#[derive(Clone, Serialize, Deserialize)]
// #[derive(Validate, Patchable)]
#[serde(try_from = "ShadowFractionDraft")]
pub struct ShadowFraction(
    //#[validate(range(0.0..=1.0))]
    f64,
);

/* --------- GENERATED ------------ */

/// Error type for [`ShadowFraction`] validation failures.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum ShadowFractionValidationError {
    /// The value is outside the valid range `[0.0, 1.0]`.
    #[error("The value must be within the range [0.0, 1.0]")]
    OutOfRange,
}

/// Draft construction of a [`ShadowFraction`].
#[derive(Serialize, Deserialize)]
pub struct ShadowFractionDraft(pub f64);

impl ShadowFractionDraft {
    /// Validate all the draft with fail fast policy where the first error found is directly returned.
    pub fn validate(&self) -> Result<(), ShadowFractionValidationError> {
        self.validate_value()
    }

    /// Validate the `inner` field.
    pub fn validate_value(&self) -> Result<(), ShadowFractionValidationError> {
        if !(0.0..=1.0).contains(&self.0) {
            return Err(ShadowFractionValidationError::OutOfRange);
        }

        Ok(())
    }
}

impl TryFrom<ShadowFractionDraft> for ShadowFraction {
    type Error = ShadowFractionValidationError;

    fn try_from(value: ShadowFractionDraft) -> Result<Self, Self::Error> {
        value.validate()?;

        Ok(Self(value.0))
    }
}

impl ShadowFraction {
    /// Create a new [`ShadowFraction`] from the given `draft`.
    /// Return the first error found during `draft` validation.
    pub fn new(draft: ShadowFractionDraft) -> Result<Self, ShadowFractionValidationError> {
        Self::try_from(draft)
    }

    // Use `value` since this is a tuple struct, otherwise use the field name
    /// Retrieve the `inner` field.
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Set the given `new_value`.
    /// Return an error if the `new_value` cannot be validated.
    pub fn set_value(&mut self, new_value: f64) -> Result<(), ShadowFractionValidationError> {
        let mut tmp_draft: ShadowFractionDraft = self.clone().into();
        tmp_draft.0 = new_value;
        let _: () = tmp_draft.validate_value()?;

        self.0 = new_value;

        Ok(())
    }
}

impl From<ShadowFraction> for ShadowFractionDraft {
    fn from(value: ShadowFraction) -> Self {
        Self(value.0)
    }
}

/* --------- WRITTEN ------------ */

/// Reference physical properties of a spacecraft used during dynamical simulations.
#[derive(Clone, Serialize, Deserialize)]
// #[derive(Validate, Patchable)]
#[serde(try_from = "SpacecraftDraft")]
// #[final_validation(validate_mass_sum)]
pub struct Spacecraft {
    /// Total spacecraft mass expressed in kilograms (kg).
    // #[validate(range(0.0..f64::INFINITY))]
    mass: f64,
    /// Mass of the spacecraft bus expressed in kilograms (kg).
    // #[validate(range(0.0..=30_000.0))]
    bus_mass: f64,
    /// Mass of the spacecraft sail expressed in kilograms (kg).
    // #[validate(range(0.0..=10_000.0))]
    sail_mass: f64,
    /// Moment of inertia matrix expressed in the body frame (kg·m²).
    // #[validate(
    //     range(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)),
    //     symmetric
    // )]
    inertia_matrix: InertiaMatrix,
    /// Fraction of sunlight reaching the spacecraft.
    sun_shadow_fraction: ShadowFraction,
    /// Celestial body this spacecraft is primarily orbiting around.
    primary_orbited_body: CelestialBodyKind,
}

/// Error type for [`Spacecraft::validate_mass_sum`] validation failures.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum SpacecraftMassSumValidationError {
    /// The spacecraft total mass is smaller than the sum of the bus and sail mass.
    #[error(
        "The spacecraft total mass must be greater or equal to the sum of the bus and sail mass"
    )]
    MassSmallerThanSum,
}

#[expect(
    clippy::multiple_inherent_impl,
    reason = "derives generate some part of the struct"
)]
impl Spacecraft {
    /// Validation function to verify that the given spacecraft `mass`
    /// is smaller than the sum of the given `bus_mass` and `sail_mass`.
    pub fn validate_mass_sum(
        draft: &SpacecraftDraft,
    ) -> Result<(), SpacecraftMassSumValidationError> {
        if draft.mass < draft.bus_mass + draft.sail_mass {
            return Err(SpacecraftMassSumValidationError::MassSmallerThanSum);
        }

        Ok(())
    }
}

/* --------- GENERATED ------------ */

/// Error type for [`Spacecraft`] validation failures.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum SpacecraftValidationError {
    /// The mass is outside the valid range `]0.0, f64::INFINITY[`.
    #[error("The mass must be within the range ]0.0, f64::INFINITY[")]
    MassOutOfRange,
    /// The bus_mass is outside the valid range `[0.0, 30_000.0[`.
    #[error("The bus_mass must be within the range [0.0, 30_000.0[")]
    BusMassOutOfRange,
    /// The sail_mass is outside the valid range `[0.0, 10_000.0[`.
    #[error("The sail_mass must be within the range [0.0, 10_000.0[")]
    SailMassOutOfRange,
    /// The validate_mass_sum validation failed.
    #[error("{0}")]
    MassSumValidationError(SpacecraftMassSumValidationError),
    /// The inertia_matrix validation failed.
    #[error("{0}")]
    InertiaMatrixSerializableValidationError(InertiaMatrixSerializableValidationError),
    /// The sun_shadow_fraction validation failed.
    #[error("{0}")]
    ShadowFractionValidationError(ShadowFractionValidationError),
}

#[derive(Serialize, Deserialize)]
/// Draft construction  of a [`Spacecraft`].
pub struct SpacecraftDraft {
    /// Total spacecraft mass expressed in kilograms (kg).
    pub mass: f64,
    /// Mass of the spacecraft bus expressed in kilograms (kg).
    pub bus_mass: f64,
    /// Mass of the spacecraft sail expressed in kilograms (kg).
    pub sail_mass: f64,
    /// Moment of inertia matrix expressed in the body frame (kg·m²).
    pub inertia_matrix: InertiaMatrixSerializableDraft, //FIXME: how to detect this???
    /// Fraction of sunlight reaching the spacecraft.
    pub sun_shadow_fraction: ShadowFractionDraft,
    /// Celestial body this spacecraft is primarily orbiting around.
    pub primary_orbited_body: CelestialBodyKind, // won;t get validated as doesn't contain #[validate] and doesn't impl Validate
}

impl SpacecraftDraft {
    /// Validate all the draft with fail fast policy where the first error found is directly returned.
    pub fn validate(&self) -> Result<(), SpacecraftValidationError> {
        self.validate_mass()?;
        self.validate_bus_mass()?;
        self.validate_sail_mass()?;
        self.validate_inertia_matrix()?;
        self.validate_sun_shadow_fraction()?;

        Spacecraft::validate_mass_sum(self)
            .map_err(SpacecraftValidationError::MassSumValidationError)?;

        Ok(())
    }

    /// Validate the `mass` field.
    pub fn validate_mass(&self) -> Result<(), SpacecraftValidationError> {
        if !(0.0..f64::INFINITY).contains(&self.mass) {
            return Err(SpacecraftValidationError::MassOutOfRange);
        }
        Ok(())
    }

    /// Validate the `bus_mass` field.
    pub fn validate_bus_mass(&self) -> Result<(), SpacecraftValidationError> {
        if !(0.0..=30_000.0).contains(&self.bus_mass) {
            return Err(SpacecraftValidationError::BusMassOutOfRange);
        }
        Ok(())
    }

    /// Validate the `sail_mass` field.
    pub fn validate_sail_mass(&self) -> Result<(), SpacecraftValidationError> {
        if !(0.0..=10_000.0).contains(&self.sail_mass) {
            return Err(SpacecraftValidationError::SailMassOutOfRange);
        }
        Ok(())
    }

    /// Validate the `inertia_matrix` field.
    pub fn validate_inertia_matrix(&self) -> Result<(), SpacecraftValidationError> {
        InertiaMatrixSerializableDraft::validate(&self.inertia_matrix)
            .map_err(SpacecraftValidationError::InertiaMatrixSerializableValidationError)?;
        Ok(())
    }

    /// Validate the `sun_shadow_fraction` field.
    pub fn validate_sun_shadow_fraction(&self) -> Result<(), SpacecraftValidationError> {
        ShadowFractionDraft::validate(&self.sun_shadow_fraction)
            .map_err(SpacecraftValidationError::ShadowFractionValidationError)?;

        Ok(())
    }
}

impl TryFrom<SpacecraftDraft> for Spacecraft {
    type Error = SpacecraftValidationError;

    fn try_from(value: SpacecraftDraft) -> Result<Self, Self::Error> {
        value.validate()?;

        let inertia_matrix_serializable: InertiaMatrixSerializable = value
            .inertia_matrix
            .try_into()
            .map_err(SpacecraftValidationError::InertiaMatrixSerializableValidationError)?;

        Ok(Self {
            mass: value.mass,
            bus_mass: value.bus_mass,
            sail_mass: value.sail_mass,
            inertia_matrix: inertia_matrix_serializable.into(),
            sun_shadow_fraction: value
                .sun_shadow_fraction
                .try_into()
                .map_err(SpacecraftValidationError::ShadowFractionValidationError)?,
            primary_orbited_body: value.primary_orbited_body,
        })
    }
}

impl Spacecraft {
    /// Create a new [`Spacecraft`] from the given `draft`.
    /// Return the first error found during `draft` validation.
    pub fn new(draft: SpacecraftDraft) -> Result<Self, SpacecraftValidationError> {
        Self::try_from(draft)
    }

    /// Retrieve the `mass` field.
    pub fn mass(&self) -> f64 {
        self.mass
    }

    /// Set the given `new_mass`.
    /// Return an error if the `new_mass` cannot be validated.
    pub fn set_mass(&mut self, new_mass: f64) -> Result<(), SpacecraftValidationError> {
        let mut tmp_draft: SpacecraftDraft = self.clone().into();
        tmp_draft.mass = new_mass;
        let _: () = tmp_draft.validate_mass()?;

        Self::validate_mass_sum(&tmp_draft)
            .map_err(SpacecraftValidationError::MassSumValidationError)?;

        self.mass = new_mass;

        Ok(())
    }
}

impl From<Spacecraft> for SpacecraftDraft {
    fn from(value: Spacecraft) -> Self {
        let inertia_matrix_serializable: InertiaMatrixSerializable = value.inertia_matrix.into();

        Self {
            mass: value.mass,
            bus_mass: value.bus_mass,
            sail_mass: value.sail_mass,
            inertia_matrix: inertia_matrix_serializable.into(),
            sun_shadow_fraction: value.sun_shadow_fraction.into(),
            primary_orbited_body: value.primary_orbited_body,
        }
    }
}
