//! A crate for validating types.

use core::ops::{Bound, RangeBounds as _};

use nalgebra::Matrix3;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// TODO: the trait will live in the runtime crate and be re-exported next to the derive macro
/// Marker trait for types constructible only from a validated draft.
pub trait Validate: Sized {
    /// Unvalidated draft mirror of the type.
    type Draft;
    /// Error produced when the draft is invalid.
    type Error;

    /// Validate the given `draft` with fail fast policy.
    fn validate(draft: &Self::Draft) -> Result<(), Self::Error>;

    /// Build the type from the given `draft` without validating it.
    /// Only meant to be called by the generated code after a successful validation.
    #[doc(hidden)]
    fn from_draft_unchecked(draft: Self::Draft) -> Self;

    /// Build the type from the given `draft`, validating it first.
    /// Return the first error found during `draft` validation.
    fn from_draft(draft: Self::Draft) -> Result<Self, Self::Error> {
        Self::validate(&draft)?;
        Ok(Self::from_draft_unchecked(draft))
    }
}

/// Marker trait for  types that can be patched field by field with validated setters.
pub trait Patch: Validate {
    /// Convert back to a draft for patching.
    fn to_draft(&self) -> Self::Draft;
}

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

/// Serde representation of a [`InertiaMatrix`].
#[derive(Clone, Serialize, Deserialize)]
// #[serde(try_from = "InertiaMatrixSerializableDraft")]
// #[derive(Validate, Patch)]
// #[final_validation(validate_realizability, error = InertiaMatrixRealizabilityValidationError)]
pub struct InertiaMatrixSerializable {
    /// Ixx.
    // #[validate(range(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)))]
    xx: f64,
    /// Ixy.
    // #[validate(finite)]
    xy: f64,
    /// Ixz.
    // #[validate(finite)]
    xz: f64,
    /// Iyx.
    // #[validate(finite)]
    yx: f64,
    /// Iyy.
    // #[validate(range(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)))]
    yy: f64,
    /// Iyz.
    // #[validate(finite)]
    yz: f64,
    /// Izx.
    // #[validate(finite)]
    zx: f64,
    /// Izy.
    // #[validate(finite)]
    zy: f64,
    /// Izz.
    // #[validate(range(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)))]
    zz: f64,
}

/// Error type for [`InertiaMatrixSerializable::validate_realizability`] validation failures.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum InertiaMatrixRealizabilityValidationError {
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

    /// Validation function to verify that the matrix of the given `draft` is symmetric
    /// within [`Self::SYMMETRY_TOLERANCE`] and corresponds to a physically realizable
    /// mass distribution within [`Self::REALIZABILITY_TOLERANCE`].
    /// The entries are expected to be finite, which is enforced by the per field validations.
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
            return Err(
                InertiaMatrixRealizabilityValidationError::CovarianceNotPositiveSemiDefinite,
            );
        }

        Ok(())
    }
}

/* --------- GENERATED ------------ */

/// Error type for [`InertiaMatrixSerializable`] validation failures.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum InertiaMatrixSerializableValidationError {
    /// The value is outside the valid range `]0.0, +inf[`.
    #[error("The value must be within the range ]0.0, +inf[")]
    OutOfRange,
    /// The value is not a finite number.
    #[error("The value must be a finite number")]
    NotFinite,
    /// The validate_realizability validation failed.
    #[error("{0}")]
    RealizabilityValidationError(InertiaMatrixRealizabilityValidationError),
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
        self.validate_xy()?;
        self.validate_xz()?;
        self.validate_yx()?;
        self.validate_yy()?;
        self.validate_yz()?;
        self.validate_zx()?;
        self.validate_zy()?;
        self.validate_zz()?;

        InertiaMatrixSerializable::validate_realizability(self)
            .map_err(InertiaMatrixSerializableValidationError::RealizabilityValidationError)?;

        Ok(())
    }

    /// Validate the `xx` field.
    pub fn validate_xx(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)).contains(&self.xx) {
            return Err(InertiaMatrixSerializableValidationError::OutOfRange);
        }

        Ok(())
    }

    /// Validate the `xy` field.
    pub fn validate_xy(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !self.xy.is_finite() {
            return Err(InertiaMatrixSerializableValidationError::NotFinite);
        }

        Ok(())
    }

    /// Validate the `xz` field.
    pub fn validate_xz(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !self.xz.is_finite() {
            return Err(InertiaMatrixSerializableValidationError::NotFinite);
        }

        Ok(())
    }

    /// Validate the `yx` field.
    pub fn validate_yx(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !self.yx.is_finite() {
            return Err(InertiaMatrixSerializableValidationError::NotFinite);
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

    /// Validate the `yz` field.
    pub fn validate_yz(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !self.yz.is_finite() {
            return Err(InertiaMatrixSerializableValidationError::NotFinite);
        }

        Ok(())
    }

    /// Validate the `zx` field.
    pub fn validate_zx(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !self.zx.is_finite() {
            return Err(InertiaMatrixSerializableValidationError::NotFinite);
        }

        Ok(())
    }

    /// Validate the `zy` field.
    pub fn validate_zy(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !self.zy.is_finite() {
            return Err(InertiaMatrixSerializableValidationError::NotFinite);
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

        Self::validate_realizability(&tmp_draft)
            .map_err(InertiaMatrixSerializableValidationError::RealizabilityValidationError)?;

        self.xx = new_xx;

        Ok(())
    }

    /// Set the given `new_xy`.
    /// Return an error if the `new_xy` cannot be validated.
    pub fn set_xy(&mut self, new_xy: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.xy = new_xy;
        let _: () = tmp_draft.validate_xy()?;

        Self::validate_realizability(&tmp_draft)
            .map_err(InertiaMatrixSerializableValidationError::RealizabilityValidationError)?;

        self.xy = new_xy;

        Ok(())
    }

    /// Set the given `new_xz`.
    /// Return an error if the `new_xz` cannot be validated.
    pub fn set_xz(&mut self, new_xz: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.xz = new_xz;
        let _: () = tmp_draft.validate_xz()?;

        Self::validate_realizability(&tmp_draft)
            .map_err(InertiaMatrixSerializableValidationError::RealizabilityValidationError)?;

        self.xz = new_xz;

        Ok(())
    }

    /// Set the given `new_yx`.
    /// Return an error if the `new_yx` cannot be validated.
    pub fn set_yx(&mut self, new_yx: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.yx = new_yx;
        let _: () = tmp_draft.validate_yx()?;

        Self::validate_realizability(&tmp_draft)
            .map_err(InertiaMatrixSerializableValidationError::RealizabilityValidationError)?;

        self.yx = new_yx;

        Ok(())
    }

    /// Set the given `new_yy`.
    /// Return an error if the `new_yy` cannot be validated.
    pub fn set_yy(&mut self, new_yy: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.yy = new_yy;
        let _: () = tmp_draft.validate_yy()?;

        Self::validate_realizability(&tmp_draft)
            .map_err(InertiaMatrixSerializableValidationError::RealizabilityValidationError)?;

        self.yy = new_yy;

        Ok(())
    }

    /// Set the given `new_yz`.
    /// Return an error if the `new_yz` cannot be validated.
    pub fn set_yz(&mut self, new_yz: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.yz = new_yz;
        let _: () = tmp_draft.validate_yz()?;

        Self::validate_realizability(&tmp_draft)
            .map_err(InertiaMatrixSerializableValidationError::RealizabilityValidationError)?;

        self.yz = new_yz;

        Ok(())
    }

    /// Set the given `new_zx`.
    /// Return an error if the `new_zx` cannot be validated.
    pub fn set_zx(&mut self, new_zx: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.zx = new_zx;
        let _: () = tmp_draft.validate_zx()?;

        Self::validate_realizability(&tmp_draft)
            .map_err(InertiaMatrixSerializableValidationError::RealizabilityValidationError)?;

        self.zx = new_zx;

        Ok(())
    }

    /// Set the given `new_zy`.
    /// Return an error if the `new_zy` cannot be validated.
    pub fn set_zy(&mut self, new_zy: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.zy = new_zy;
        let _: () = tmp_draft.validate_zy()?;

        Self::validate_realizability(&tmp_draft)
            .map_err(InertiaMatrixSerializableValidationError::RealizabilityValidationError)?;

        self.zy = new_zy;

        Ok(())
    }

    /// Set the given `new_zz`.
    /// Return an error if the `new_zz` cannot be validated.
    pub fn set_zz(&mut self, new_zz: f64) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut tmp_draft: InertiaMatrixSerializableDraft = self.clone().into();
        tmp_draft.zz = new_zz;
        let _: () = tmp_draft.validate_zz()?;

        Self::validate_realizability(&tmp_draft)
            .map_err(InertiaMatrixSerializableValidationError::RealizabilityValidationError)?;

        self.zz = new_zz;

        Ok(())
    }
}

impl TryFrom<InertiaMatrixSerializableDraft> for InertiaMatrixSerializable {
    type Error = InertiaMatrixSerializableValidationError;

    fn try_from(value: InertiaMatrixSerializableDraft) -> Result<Self, Self::Error> {
        Self::from_draft(value)
    }
}

impl Validate for InertiaMatrixSerializable {
    type Draft = InertiaMatrixSerializableDraft;
    type Error = InertiaMatrixSerializableValidationError;

    fn validate(draft: &Self::Draft) -> Result<(), Self::Error> {
        draft.validate()
    }

    fn from_draft_unchecked(draft: Self::Draft) -> Self {
        Self {
            xx: draft.xx,
            xy: draft.xy,
            xz: draft.xz,
            yx: draft.yx,
            yy: draft.yy,
            yz: draft.yz,
            zx: draft.zx,
            zy: draft.zy,
            zz: draft.zz,
        }
    }
}

impl Patch for InertiaMatrixSerializable {
    fn to_draft(&self) -> Self::Draft {
        self.clone().into()
    }
}

/* --------- WRITTEN ------------ */

/// Fraction of sunlight reaching a spacecraft, bounded to [0.0, 1.0].
///
/// A value of 1.0 represents full sunlight and 0.0 represents full eclipse.
#[repr(transparent)]
#[derive(Clone, Serialize, Deserialize)]
// #[derive(Validate, Patch)]
#[serde(try_from = "ShadowFractionDraft")]
pub struct ShadowFraction(
    // #[validate(range(0.0..=1.0))]
    f64,
);

/* --------- GENERATED ------------ */

/// Error type for [`ShadowFraction`] validation failures.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
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

    /// Validate the `value` field.
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
        Self::from_draft(value)
    }
}

impl ShadowFraction {
    /// Create a new [`ShadowFraction`] from the given `draft`.
    /// Return the first error found during `draft` validation.
    pub fn new(draft: ShadowFractionDraft) -> Result<Self, ShadowFractionValidationError> {
        Self::try_from(draft)
    }

    // Use `value` since this is a tuple struct, otherwise use the field name
    /// Retrieve the `value` field.
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

impl Validate for ShadowFraction {
    type Draft = ShadowFractionDraft;
    type Error = ShadowFractionValidationError;

    fn validate(draft: &Self::Draft) -> Result<(), Self::Error> {
        draft.validate()
    }

    fn from_draft_unchecked(draft: Self::Draft) -> Self {
        Self(draft.0)
    }
}

impl Patch for ShadowFraction {
    fn to_draft(&self) -> Self::Draft {
        self.clone().into()
    }
}

/* --------- WRITTEN ------------ */

/// Reference physical properties of a spacecraft used during dynamical simulations.
#[derive(Clone, Serialize, Deserialize)]
// #[derive(Validate, Patch)]
#[serde(try_from = "SpacecraftDraft")]
// #[final_validation(validate_mass_sum, error = SpacecraftMassSumValidationError)]
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
    // #[validate(nested)]
    inertia_matrix: InertiaMatrix,
    /// Fraction of sunlight reaching the spacecraft.
    // #[validate(nested)]
    sun_shadow_fraction: ShadowFraction,
    /// Celestial body this spacecraft is primarily orbiting around.
    // #[validate(skip)]
    primary_orbited_body: CelestialBodyKind,
}

/// Error type for [`Spacecraft::validate_mass_sum`] validation failures.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
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
    /// Validation function to verify that the `mass` of the given `draft`
    /// is greater than or equal to the sum of its `bus_mass` and `sail_mass`.
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
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum SpacecraftValidationError {
    /// The mass is outside the valid range `[0.0, f64::INFINITY[`.
    #[error("The mass must be within the range [0.0, f64::INFINITY[")]
    MassOutOfRange,
    /// The bus_mass is outside the valid range `[0.0, 30_000.0]`.
    #[error("The bus_mass must be within the range [0.0, 30_000.0]")]
    BusMassOutOfRange,
    /// The sail_mass is outside the valid range `[0.0, 10_000.0]`.
    #[error("The sail_mass must be within the range [0.0, 10_000.0]")]
    SailMassOutOfRange,
    /// The validate_mass_sum validation failed.
    #[error("{0}")]
    MassSumValidationError(SpacecraftMassSumValidationError),
    /// The inertia_matrix validation failed.
    #[error("{0}")]
    InertiaMatrixValidationError(<InertiaMatrix as Validate>::Error),
    /// The sun_shadow_fraction validation failed.
    #[error("{0}")]
    SunShadowFractionValidationError(<ShadowFraction as Validate>::Error),
}

#[derive(Serialize, Deserialize)]
/// Draft construction of a [`Spacecraft`].
pub struct SpacecraftDraft {
    /// Total spacecraft mass expressed in kilograms (kg).
    pub mass: f64,
    /// Mass of the spacecraft bus expressed in kilograms (kg).
    pub bus_mass: f64,
    /// Mass of the spacecraft sail expressed in kilograms (kg).
    pub sail_mass: f64,
    /// Moment of inertia matrix expressed in the body frame (kg·m²).
    pub inertia_matrix: <InertiaMatrix as Validate>::Draft,
    /// Fraction of sunlight reaching the spacecraft.
    pub sun_shadow_fraction: <ShadowFraction as Validate>::Draft,
    /// Celestial body this spacecraft is primarily orbiting around.
    pub primary_orbited_body: CelestialBodyKind, // #[validate(skip)] fields are passed through verbatim
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
        <InertiaMatrix as Validate>::validate(&self.inertia_matrix)
            .map_err(SpacecraftValidationError::InertiaMatrixValidationError)?;
        Ok(())
    }

    /// Validate the `sun_shadow_fraction` field.
    pub fn validate_sun_shadow_fraction(&self) -> Result<(), SpacecraftValidationError> {
        <ShadowFraction as Validate>::validate(&self.sun_shadow_fraction)
            .map_err(SpacecraftValidationError::SunShadowFractionValidationError)?;

        Ok(())
    }
}

impl TryFrom<SpacecraftDraft> for Spacecraft {
    type Error = SpacecraftValidationError;

    fn try_from(value: SpacecraftDraft) -> Result<Self, Self::Error> {
        Self::from_draft(value)
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

    /// Retrieve the `bus_mass` field.
    pub fn bus_mass(&self) -> f64 {
        self.bus_mass
    }

    /// Retrieve the `sail_mass` field.
    pub fn sail_mass(&self) -> f64 {
        self.sail_mass
    }

    // The macro cannot detect Copy so primitives are special-cased by token
    // and everything else is returned by reference
    /// Retrieve the `inertia_matrix` field.
    pub fn inertia_matrix(&self) -> &InertiaMatrix {
        &self.inertia_matrix
    }

    /// Retrieve the `sun_shadow_fraction` field.
    pub fn sun_shadow_fraction(&self) -> &ShadowFraction {
        &self.sun_shadow_fraction
    }

    /// Retrieve the `primary_orbited_body` field.
    pub fn primary_orbited_body(&self) -> &CelestialBodyKind {
        &self.primary_orbited_body
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

    /// Set the given `new_bus_mass`.
    /// Return an error if the `new_bus_mass` cannot be validated.
    pub fn set_bus_mass(&mut self, new_bus_mass: f64) -> Result<(), SpacecraftValidationError> {
        let mut tmp_draft: SpacecraftDraft = self.clone().into();
        tmp_draft.bus_mass = new_bus_mass;
        let _: () = tmp_draft.validate_bus_mass()?;

        Self::validate_mass_sum(&tmp_draft)
            .map_err(SpacecraftValidationError::MassSumValidationError)?;

        self.bus_mass = new_bus_mass;

        Ok(())
    }

    /// Set the given `new_sail_mass`.
    /// Return an error if the `new_sail_mass` cannot be validated.
    pub fn set_sail_mass(&mut self, new_sail_mass: f64) -> Result<(), SpacecraftValidationError> {
        let mut tmp_draft: SpacecraftDraft = self.clone().into();
        tmp_draft.sail_mass = new_sail_mass;
        let _: () = tmp_draft.validate_sail_mass()?;

        Self::validate_mass_sum(&tmp_draft)
            .map_err(SpacecraftValidationError::MassSumValidationError)?;

        self.sail_mass = new_sail_mass;

        Ok(())
    }

    /// Set the given `new_inertia_matrix`.
    /// Return an error if the `new_inertia_matrix` cannot be validated.
    pub fn set_inertia_matrix(
        &mut self,
        new_inertia_matrix: InertiaMatrix,
    ) -> Result<(), SpacecraftValidationError> {
        let mut tmp_draft: SpacecraftDraft = self.clone().into();
        tmp_draft.inertia_matrix = new_inertia_matrix.to_draft();
        let _: () = tmp_draft.validate_inertia_matrix()?;

        Self::validate_mass_sum(&tmp_draft)
            .map_err(SpacecraftValidationError::MassSumValidationError)?;

        self.inertia_matrix = new_inertia_matrix;

        Ok(())
    }

    /// Set the given `new_sun_shadow_fraction`.
    /// Return an error if the `new_sun_shadow_fraction` cannot be validated.
    pub fn set_sun_shadow_fraction(
        &mut self,
        new_sun_shadow_fraction: ShadowFraction,
    ) -> Result<(), SpacecraftValidationError> {
        let mut tmp_draft: SpacecraftDraft = self.clone().into();
        tmp_draft.sun_shadow_fraction = new_sun_shadow_fraction.to_draft();
        let _: () = tmp_draft.validate_sun_shadow_fraction()?;

        Self::validate_mass_sum(&tmp_draft)
            .map_err(SpacecraftValidationError::MassSumValidationError)?;

        self.sun_shadow_fraction = new_sun_shadow_fraction;

        Ok(())
    }

    /// Set the given `new_primary_orbited_body`.
    /// Return an error if the `new_primary_orbited_body` cannot be validated.
    pub fn set_primary_orbited_body(
        &mut self,
        new_primary_orbited_body: CelestialBodyKind,
    ) -> Result<(), SpacecraftValidationError> {
        let mut tmp_draft: SpacecraftDraft = self.clone().into();
        tmp_draft.primary_orbited_body = new_primary_orbited_body.clone();

        Self::validate_mass_sum(&tmp_draft)
            .map_err(SpacecraftValidationError::MassSumValidationError)?;

        self.primary_orbited_body = new_primary_orbited_body;

        Ok(())
    }
}

impl From<Spacecraft> for SpacecraftDraft {
    fn from(value: Spacecraft) -> Self {
        Self {
            mass: value.mass,
            bus_mass: value.bus_mass,
            sail_mass: value.sail_mass,
            inertia_matrix: value.inertia_matrix.to_draft(),
            sun_shadow_fraction: value.sun_shadow_fraction.to_draft(),
            primary_orbited_body: value.primary_orbited_body,
        }
    }
}

impl Validate for Spacecraft {
    type Draft = SpacecraftDraft;
    type Error = SpacecraftValidationError;

    fn validate(draft: &Self::Draft) -> Result<(), Self::Error> {
        draft.validate()
    }

    fn from_draft_unchecked(draft: Self::Draft) -> Self {
        Self {
            mass: draft.mass,
            bus_mass: draft.bus_mass,
            sail_mass: draft.sail_mass,
            inertia_matrix: InertiaMatrix::from_draft_unchecked(draft.inertia_matrix),
            sun_shadow_fraction: ShadowFraction::from_draft_unchecked(draft.sun_shadow_fraction),
            primary_orbited_body: draft.primary_orbited_body,
        }
    }
}

impl Patch for Spacecraft {
    fn to_draft(&self) -> Self::Draft {
        self.clone().into()
    }
}
