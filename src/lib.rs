//! A crate for validating types.

#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(all(feature = "math-native", not(feature = "std")))]
compile_error!("Feature \"math-native\" requires the \"std\" feature");

#[cfg(all(feature = "math-native", feature = "math-libm"))]
compile_error!("Features \"math-native\" and \"math-libm\" are mutually exclusive");

#[cfg(not(any(feature = "math-native", feature = "math-libm")))]
compile_error!("Either \"math-native\" or \"math-libm\" feature must be enabled");

use core::{
    fmt,
    ops::{Bound, RangeBounds as _},
};

use nalgebra::{ComplexField, Matrix3};
#[cfg(feature = "std")]
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
#[derive(Clone)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
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

#[cfg(feature = "std")]
impl Serialize for InertiaMatrix {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        InertiaMatrixSerializable::from(self.clone()).serialize(serializer)
    }
}

#[cfg(feature = "std")]
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
#[derive(Clone)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "std", serde(try_from = "InertiaMatrixSerializableDraft"))]
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

        // The qualified ComplexField calls make the fused multiply add follow the selected
        // math feature, the inherent f64 method would always resolve to the native one
        // Order 2: the three 2x2 principal minors.
        let minor_2_ok =
            ComplexField::mul_add(s01, -s01, s00 * s11) >= -Self::REALIZABILITY_TOLERANCE
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

/* --------- GENERATED ------------ */

// One field enum is generated per type with a variant per range or finite validated field,
// the shared OutOfRange and NotFinite variants carry it to name the failing field
/// Validated fields of a [`InertiaMatrixSerializable`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InertiaMatrixSerializableField {
    /// The `xx` field.
    Xx,
    /// The `xy` field.
    Xy,
    /// The `xz` field.
    Xz,
    /// The `yx` field.
    Yx,
    /// The `yy` field.
    Yy,
    /// The `yz` field.
    Yz,
    /// The `zx` field.
    Zx,
    /// The `zy` field.
    Zy,
    /// The `zz` field.
    Zz,
}

impl fmt::Display for InertiaMatrixSerializableField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Xx => "xx",
            Self::Xy => "xy",
            Self::Xz => "xz",
            Self::Yx => "yx",
            Self::Yy => "yy",
            Self::Yz => "yz",
            Self::Zx => "zx",
            Self::Zy => "zy",
            Self::Zz => "zz",
        })
    }
}

/// Error type for [`InertiaMatrixSerializable`] validation failures.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum InertiaMatrixSerializableValidationError {
    // TODO: find a fix for the `variant-size-differences`lint
    /// The field value is outside its valid range.
    #[error("The {field} must be within the range {range}")]
    OutOfRange {
        /// The field that failed the validation.
        field: InertiaMatrixSerializableField,
        /// The valid range of the field.
        range: &'static str,
    },
    /// The field value is not a finite number.
    #[error("The {field} must be a finite number")]
    NotFinite {
        /// The field that failed the validation.
        field: InertiaMatrixSerializableField,
    },
    /// The validate_realizability validation failed.
    #[error("{0}")]
    RealizabilityValidationError(InertiaMatrixRealizabilityValidationError),
}

/// Draft construction of a [`InertiaMatrixSerializable`].
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct InertiaMatrixSerializableDraft {
    /// Ixx.
    pub xx: f64,
    /// Ixy.
    pub xy: f64,
    /// Ixz.
    pub xz: f64,
    /// Iyx.
    pub yx: f64,
    /// Iyy.
    pub yy: f64,
    /// Iyz.
    pub yz: f64,
    /// Izx.
    pub zx: f64,
    /// Izy.
    pub zy: f64,
    /// Izz.
    pub zz: f64,
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
            return Err(InertiaMatrixSerializableValidationError::OutOfRange {
                field: InertiaMatrixSerializableField::Xx,
                range: "]0.0, +inf[",
            });
        }

        Ok(())
    }

    /// Validate the `xy` field.
    pub fn validate_xy(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !self.xy.is_finite() {
            return Err(InertiaMatrixSerializableValidationError::NotFinite {
                field: InertiaMatrixSerializableField::Xy,
            });
        }

        Ok(())
    }

    /// Validate the `xz` field.
    pub fn validate_xz(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !self.xz.is_finite() {
            return Err(InertiaMatrixSerializableValidationError::NotFinite {
                field: InertiaMatrixSerializableField::Xz,
            });
        }

        Ok(())
    }

    /// Validate the `yx` field.
    pub fn validate_yx(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !self.yx.is_finite() {
            return Err(InertiaMatrixSerializableValidationError::NotFinite {
                field: InertiaMatrixSerializableField::Yx,
            });
        }

        Ok(())
    }

    /// Validate the `yy` field.
    pub fn validate_yy(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)).contains(&self.yy) {
            return Err(InertiaMatrixSerializableValidationError::OutOfRange {
                field: InertiaMatrixSerializableField::Yy,
                range: "]0.0, +inf[",
            });
        }

        Ok(())
    }

    /// Validate the `yz` field.
    pub fn validate_yz(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !self.yz.is_finite() {
            return Err(InertiaMatrixSerializableValidationError::NotFinite {
                field: InertiaMatrixSerializableField::Yz,
            });
        }

        Ok(())
    }

    /// Validate the `zx` field.
    pub fn validate_zx(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !self.zx.is_finite() {
            return Err(InertiaMatrixSerializableValidationError::NotFinite {
                field: InertiaMatrixSerializableField::Zx,
            });
        }

        Ok(())
    }

    /// Validate the `zy` field.
    pub fn validate_zy(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !self.zy.is_finite() {
            return Err(InertiaMatrixSerializableValidationError::NotFinite {
                field: InertiaMatrixSerializableField::Zy,
            });
        }

        Ok(())
    }

    /// Validate the `zz` field.
    pub fn validate_zz(&self) -> Result<(), InertiaMatrixSerializableValidationError> {
        if !(Bound::Excluded(0.0_f64), Bound::Excluded(f64::INFINITY)).contains(&self.zz) {
            return Err(InertiaMatrixSerializableValidationError::OutOfRange {
                field: InertiaMatrixSerializableField::Zz,
                range: "]0.0, +inf[",
            });
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
#[derive(Clone)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
// #[derive(Validate, Patch)]
#[cfg_attr(feature = "std", serde(try_from = "ShadowFractionDraft"))]
pub struct ShadowFraction(
    // #[validate(range(0.0..=1.0))]
    f64,
);

/* --------- GENERATED ------------ */

/// Validated fields of a [`ShadowFraction`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShadowFractionField {
    // Use `Value` since this is a tuple struct, otherwise use the field name
    /// The `value` field.
    Value,
}

impl fmt::Display for ShadowFractionField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Value => "value",
        })
    }
}

/// Error type for [`ShadowFraction`] validation failures.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum ShadowFractionValidationError {
    /// The field value is outside its valid range.
    #[error("The {field} must be within the range {range}")]
    OutOfRange {
        /// The field that failed the validation.
        field: ShadowFractionField,
        /// The valid range of the field.
        range: &'static str,
    },
}

/// Draft construction of a [`ShadowFraction`].
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ShadowFractionDraft(pub f64);

impl ShadowFractionDraft {
    /// Validate all the draft with fail fast policy where the first error found is directly returned.
    pub fn validate(&self) -> Result<(), ShadowFractionValidationError> {
        self.validate_value()
    }

    /// Validate the `value` field.
    pub fn validate_value(&self) -> Result<(), ShadowFractionValidationError> {
        if !(0.0..=1.0).contains(&self.0) {
            return Err(ShadowFractionValidationError::OutOfRange {
                field: ShadowFractionField::Value,
                range: "[0.0, 1.0]",
            });
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
#[derive(Clone)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
// #[derive(Validate, Patch)]
#[cfg_attr(feature = "std", serde(try_from = "SpacecraftDraft"))]
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
    // a skip field must not be read by any final validation function
    primary_orbited_body: CelestialBodyKind,
}

/// Error type for [`Spacecraft::validate_mass_sum`] validation failures.
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
impl Spacecraft {
    /// Absolute tolerance for the mass sum check, in kilograms (kg).
    pub const MASS_SUM_TOLERANCE: f64 = 1e-9;

    /// Validation function to verify that the `mass` of the given `draft` is greater than
    /// or equal to the sum of its `bus_mass` and `sail_mass` within [`Self::MASS_SUM_TOLERANCE`].
    pub fn validate_mass_sum(
        draft: &SpacecraftDraft,
    ) -> Result<(), SpacecraftMassSumValidationError> {
        if draft.mass < draft.bus_mass + draft.sail_mass - Self::MASS_SUM_TOLERANCE {
            return Err(SpacecraftMassSumValidationError::MassSmallerThanSum);
        }

        Ok(())
    }
}

/* --------- GENERATED ------------ */

/// Validated fields of a [`Spacecraft`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpacecraftField {
    /// The `mass` field.
    Mass,
    /// The `bus_mass` field.
    BusMass,
    /// The `sail_mass` field.
    SailMass,
}

impl fmt::Display for SpacecraftField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Mass => "mass",
            Self::BusMass => "bus_mass",
            Self::SailMass => "sail_mass",
        })
    }
}

/// Error type for [`Spacecraft`] validation failures.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum SpacecraftValidationError {
    /// The field value is outside its valid range.
    #[error("The {field} must be within the range {range}")]
    OutOfRange {
        /// The field that failed the validation.
        field: SpacecraftField,
        /// The valid range of the field.
        range: &'static str,
    },
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

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
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
    // #[validate(skip)] fields are passed through verbatim and are excluded from the final validation
    pub primary_orbited_body: CelestialBodyKind,
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
            return Err(SpacecraftValidationError::OutOfRange {
                field: SpacecraftField::Mass,
                range: "[0.0, f64::INFINITY[",
            });
        }
        Ok(())
    }

    /// Validate the `bus_mass` field.
    pub fn validate_bus_mass(&self) -> Result<(), SpacecraftValidationError> {
        if !(0.0..=30_000.0).contains(&self.bus_mass) {
            return Err(SpacecraftValidationError::OutOfRange {
                field: SpacecraftField::BusMass,
                range: "[0.0, 30_000.0]",
            });
        }
        Ok(())
    }

    /// Validate the `sail_mass` field.
    pub fn validate_sail_mass(&self) -> Result<(), SpacecraftValidationError> {
        if !(0.0..=10_000.0).contains(&self.sail_mass) {
            return Err(SpacecraftValidationError::OutOfRange {
                field: SpacecraftField::SailMass,
                range: "[0.0, 10_000.0]",
            });
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

    // #[validate(skip)] fields get an infallible setter with no validation at all,
    // the skip contract guarantees the field takes part in no field or final validation.
    /// Set the given `new_primary_orbited_body`.
    pub fn set_primary_orbited_body(&mut self, new_primary_orbited_body: CelestialBodyKind) {
        self.primary_orbited_body = new_primary_orbited_body;
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

/* --------- TESTS ------------ */

#[cfg(test)]
mod tests {
    use crate::{
        CelestialBodyKind, InertiaMatrixSerializableDraft, InertiaMatrixSerializableField,
        InertiaMatrixSerializableValidationError, ShadowFractionDraft, SpacecraftDraft,
    };

    // Mark serde_json as used so `unused_crate_dependencies` stays quiet when the std feature
    // is disabled and the serde integration tests are compiled out
    #[cfg(not(feature = "std"))]
    use serde_json as _;

    /// Valid inertia draft with a diagonal of 2.0, 3.0 and 4.0 kg·m².
    const VALID_INERTIA_DRAFT: InertiaMatrixSerializableDraft =
        diagonal_inertia_draft(2.0, 3.0, 4.0);

    /// Valid spacecraft draft of 1000.0 kg with a 600.0 kg bus and a 300.0 kg sail.
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
    /// Used to exercise every field validator individually.
    type InertiaFieldCase = (
        &'static str,
        fn(&mut InertiaMatrixSerializableDraft, f64),
        fn(&InertiaMatrixSerializableDraft) -> Result<(), InertiaMatrixSerializableValidationError>,
        InertiaMatrixSerializableField,
    );

    /// Build an inertia draft with the given `xx`, `yy` and `zz` diagonal and zero off-diagonals.
    const fn diagonal_inertia_draft(xx: f64, yy: f64, zz: f64) -> InertiaMatrixSerializableDraft {
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
            inertia_matrix: VALID_INERTIA_DRAFT,
            sun_shadow_fraction: ShadowFractionDraft(0.5),
            primary_orbited_body: CelestialBodyKind::Earth,
        }
    }

    /// Assert that `actual` is exactly `expected`, `described_as` naming the compared quantity.
    /// The bit patterns are compared so the check stays exact and never compares floats directly.
    fn assert_float_eq(actual: f64, expected: f64, described_as: &str) {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "The {described_as} must be exactly {expected} but was {actual}"
        );
    }

    /// Write `new_value` in the inertia draft field selected by `set_field` on an otherwise valid
    /// draft and return the result of the matching per field validator `validate_field`.
    fn validate_inertia_field(
        set_field: fn(&mut InertiaMatrixSerializableDraft, f64),
        validate_field: fn(
            &InertiaMatrixSerializableDraft,
        ) -> Result<(), InertiaMatrixSerializableValidationError>,
        new_value: f64,
    ) -> Result<(), InertiaMatrixSerializableValidationError> {
        let mut draft = VALID_INERTIA_DRAFT;
        set_field(&mut draft, new_value);
        validate_field(&draft)
    }

    /// Per field validation of every validated type.
    mod field_validation {
        /// Bounds of the shadow fraction value.
        mod shadow_fraction {
            use crate::{ShadowFractionDraft, ShadowFractionField, ShadowFractionValidationError};

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

        /// Bounds of the three diagonal inertia entries, valid over the open range ]0, +inf[.
        mod inertia_matrix_diagonal {
            use super::super::{DIAGONAL_INERTIA_FIELD_CASES, validate_inertia_field};
            use crate::InertiaMatrixSerializableValidationError;

            #[test]
            fn rejects_zero() {
                for (field_name, set_field, validate_field, expected_field) in
                    DIAGONAL_INERTIA_FIELD_CASES
                {
                    assert_eq!(
                        validate_inertia_field(set_field, validate_field, 0.0),
                        Err(InertiaMatrixSerializableValidationError::OutOfRange {
                            field: expected_field,
                            range: "]0.0, +inf[",
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
                            range: "]0.0, +inf[",
                        }),
                        "A negative value must be rejected for the {field_name} field"
                    );
                }
            }

            #[test]
            fn rejects_positive_infinity() {
                for (field_name, set_field, validate_field, expected_field) in
                    DIAGONAL_INERTIA_FIELD_CASES
                {
                    assert_eq!(
                        validate_inertia_field(set_field, validate_field, f64::INFINITY),
                        Err(InertiaMatrixSerializableValidationError::OutOfRange {
                            field: expected_field,
                            range: "]0.0, +inf[",
                        }),
                        "The excluded positive infinity must be rejected for the {field_name} field"
                    );
                }
            }

            #[test]
            fn rejects_nan() {
                for (field_name, set_field, validate_field, expected_field) in
                    DIAGONAL_INERTIA_FIELD_CASES
                {
                    assert_eq!(
                        validate_inertia_field(set_field, validate_field, f64::NAN),
                        Err(InertiaMatrixSerializableValidationError::OutOfRange {
                            field: expected_field,
                            range: "]0.0, +inf[",
                        }),
                        "A not a number value must be rejected for the {field_name} field"
                    );
                }
            }
        }

        /// Finiteness of the six off-diagonal inertia entries.
        mod inertia_matrix_off_diagonal {
            use super::super::{OFF_DIAGONAL_INERTIA_FIELD_CASES, validate_inertia_field};
            use crate::InertiaMatrixSerializableValidationError;

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
            fn rejects_nan() {
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

        /// Bounds of the spacecraft masses and wrapping of the nested field errors.
        mod spacecraft_masses {
            use super::super::{
                VALID_SPACECRAFT_DRAFT, diagonal_inertia_draft, spacecraft_draft_with_masses,
            };
            use crate::{
                InertiaMatrixSerializableField, InertiaMatrixSerializableValidationError,
                ShadowFractionDraft, ShadowFractionField, ShadowFractionValidationError,
                SpacecraftDraft, SpacecraftField, SpacecraftValidationError,
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
                        range: "[0.0, f64::INFINITY[",
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
                        range: "[0.0, f64::INFINITY[",
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
                        range: "[0.0, f64::INFINITY[",
                    }),
                    "A not a number total mass must be rejected"
                );
            }

            #[test]
            fn bus_mass_accepts_upper_bound() {
                assert_eq!(
                    spacecraft_draft_with_masses(1000.0, 30_000.0, 300.0).validate_bus_mass(),
                    Ok(()),
                    "The included upper bound of the bus mass must be accepted"
                );
            }

            #[test]
            fn bus_mass_rejects_above_upper_bound() {
                assert_eq!(
                    spacecraft_draft_with_masses(1000.0, 30_000.1, 300.0).validate_bus_mass(),
                    Err(SpacecraftValidationError::OutOfRange {
                        field: SpacecraftField::BusMass,
                        range: "[0.0, 30_000.0]",
                    }),
                    "A bus mass above the upper bound must be rejected"
                );
            }

            #[test]
            fn bus_mass_rejects_negative() {
                assert_eq!(
                    spacecraft_draft_with_masses(1000.0, -1.0, 300.0).validate_bus_mass(),
                    Err(SpacecraftValidationError::OutOfRange {
                        field: SpacecraftField::BusMass,
                        range: "[0.0, 30_000.0]",
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
                            range: "]0.0, +inf[",
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

        /// Ordering guarantees of the fail fast validation.
        mod fail_fast_order {
            use super::super::{
                VALID_INERTIA_DRAFT, VALID_SPACECRAFT_DRAFT, diagonal_inertia_draft,
            };
            use crate::{
                InertiaMatrixSerializableField, InertiaMatrixSerializableValidationError,
                SpacecraftDraft, SpacecraftField, SpacecraftValidationError,
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
                        range: "]0.0, +inf[",
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
                        range: "[0.0, f64::INFINITY[",
                    }),
                    "The mass field is declared first so its error must be reported"
                );
            }
        }
    }

    /// Final validation functions called directly on hand built drafts.
    mod final_validation {
        /// Symmetry and physical realizability of an inertia matrix draft.
        mod realizability {
            use super::super::{VALID_INERTIA_DRAFT, diagonal_inertia_draft};
            use crate::{InertiaMatrixRealizabilityValidationError, InertiaMatrixSerializable};

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
                draft.xy = InertiaMatrixSerializable::SYMMETRY_TOLERANCE;
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
            fn rejects_triangle_inequality_violation() {
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
            use super::super::spacecraft_draft_with_masses;
            use crate::{Spacecraft, SpacecraftMassSumValidationError};

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
        use super::{
            VALID_INERTIA_DRAFT, VALID_SPACECRAFT_DRAFT, assert_float_eq, diagonal_inertia_draft,
            spacecraft_draft_with_masses,
        };
        use crate::{
            CelestialBodyKind, InertiaMatrixSerializable, InertiaMatrixSerializableField,
            InertiaMatrixSerializableValidationError, ShadowFraction, ShadowFractionDraft,
            ShadowFractionField, ShadowFractionValidationError, Spacecraft,
            SpacecraftMassSumValidationError, SpacecraftValidationError, Validate as _,
        };

        #[test]
        fn serializable_new_accepts_valid_draft_and_preserves_fields() {
            let matrix = InertiaMatrixSerializable::new(VALID_INERTIA_DRAFT)
                .expect("The valid inertia draft must build an inertia matrix");
            let expected = VALID_INERTIA_DRAFT;

            assert_float_eq(matrix.xx(), expected.xx, "xx field of the built matrix");
            assert_float_eq(matrix.xy(), expected.xy, "xy field of the built matrix");
            assert_float_eq(matrix.xz(), expected.xz, "xz field of the built matrix");
            assert_float_eq(matrix.yx(), expected.yx, "yx field of the built matrix");
            assert_float_eq(matrix.yy(), expected.yy, "yy field of the built matrix");
            assert_float_eq(matrix.yz(), expected.yz, "yz field of the built matrix");
            assert_float_eq(matrix.zx(), expected.zx, "zx field of the built matrix");
            assert_float_eq(matrix.zy(), expected.zy, "zy field of the built matrix");
            assert_float_eq(matrix.zz(), expected.zz, "zz field of the built matrix");
        }

        #[test]
        fn serializable_new_returns_first_error() {
            let mut draft = diagonal_inertia_draft(0.0, 3.0, 4.0);
            draft.xy = f64::NAN;

            assert_eq!(
                InertiaMatrixSerializable::new(draft).err(),
                Some(InertiaMatrixSerializableValidationError::OutOfRange {
                    field: InertiaMatrixSerializableField::Xx,
                    range: "]0.0, +inf[",
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
            assert_float_eq(
                spacecraft.inertia_matrix().0.m11,
                2.0,
                "xx entry of the nested inertia matrix",
            );
            assert_float_eq(
                spacecraft.inertia_matrix().0.m33,
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
        fn from_draft_unchecked_skips_validation() {
            let matrix = InertiaMatrixSerializable::from_draft_unchecked(diagonal_inertia_draft(
                0.0, 3.0, 4.0,
            ));

            assert_float_eq(
                matrix.xx(),
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
    }

    /// Conversions between the wrapper, its serde representation and its draft.
    mod conversion {
        use super::assert_float_eq;
        use crate::{
            InertiaMatrix, InertiaMatrixSerializable, InertiaMatrixSerializableDraft, Patch as _,
            Validate as _,
        };

        /// Build an inertia draft where every entry differs, exposing any transposition.
        fn distinct_inertia_draft() -> InertiaMatrixSerializableDraft {
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

            assert_float_eq(matrix.0.m11, 1.0, "m11 entry holding xx");
            assert_float_eq(matrix.0.m12, 2.0, "m12 entry holding xy");
            assert_float_eq(matrix.0.m13, 3.0, "m13 entry holding xz");
            assert_float_eq(matrix.0.m21, 4.0, "m21 entry holding yx");
            assert_float_eq(matrix.0.m22, 5.0, "m22 entry holding yy");
            assert_float_eq(matrix.0.m23, 6.0, "m23 entry holding yz");
            assert_float_eq(matrix.0.m31, 7.0, "m31 entry holding zx");
            assert_float_eq(matrix.0.m32, 8.0, "m32 entry holding zy");
            assert_float_eq(matrix.0.m33, 9.0, "m33 entry holding zz");
        }

        #[test]
        fn inertia_round_trip_is_lossless() {
            let original = InertiaMatrix::from_draft_unchecked(distinct_inertia_draft());
            let restored = InertiaMatrix::from(InertiaMatrixSerializable::from(original));

            assert_float_eq(restored.0.m11, 1.0, "m11 entry after the round trip");
            assert_float_eq(restored.0.m12, 2.0, "m12 entry after the round trip");
            assert_float_eq(restored.0.m13, 3.0, "m13 entry after the round trip");
            assert_float_eq(restored.0.m21, 4.0, "m21 entry after the round trip");
            assert_float_eq(restored.0.m22, 5.0, "m22 entry after the round trip");
            assert_float_eq(restored.0.m23, 6.0, "m23 entry after the round trip");
            assert_float_eq(restored.0.m31, 7.0, "m31 entry after the round trip");
            assert_float_eq(restored.0.m32, 8.0, "m32 entry after the round trip");
            assert_float_eq(restored.0.m33, 9.0, "m33 entry after the round trip");
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
        use super::{
            VALID_INERTIA_DRAFT, VALID_SPACECRAFT_DRAFT, assert_float_eq, diagonal_inertia_draft,
        };
        use crate::{
            CelestialBodyKind, InertiaMatrix, InertiaMatrixRealizabilityValidationError,
            InertiaMatrixSerializable, InertiaMatrixSerializableField,
            InertiaMatrixSerializableValidationError, Patch as _, ShadowFraction,
            ShadowFractionDraft, ShadowFractionField, ShadowFractionValidationError, Spacecraft,
            SpacecraftMassSumValidationError, SpacecraftValidationError, Validate as _,
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
            assert_float_eq(matrix.xx(), 2.5, "xx field after the accepted update");
        }

        #[test]
        fn diagonal_setter_rejects_and_leaves_state_unchanged() {
            let mut matrix = InertiaMatrixSerializable::new(VALID_INERTIA_DRAFT)
                .expect("The valid inertia draft must build an inertia matrix");

            assert_eq!(
                matrix.set_xx(0.0),
                Err(InertiaMatrixSerializableValidationError::OutOfRange {
                    field: InertiaMatrixSerializableField::Xx,
                    range: "]0.0, +inf[",
                }),
                "An xx update to the excluded lower bound must be rejected"
            );

            let expected = VALID_INERTIA_DRAFT;
            assert_float_eq(matrix.xx(), expected.xx, "xx field after the rejection");
            assert_float_eq(matrix.xy(), expected.xy, "xy field after the rejection");
            assert_float_eq(matrix.xz(), expected.xz, "xz field after the rejection");
            assert_float_eq(matrix.yx(), expected.yx, "yx field after the rejection");
            assert_float_eq(matrix.yy(), expected.yy, "yy field after the rejection");
            assert_float_eq(matrix.yz(), expected.yz, "yz field after the rejection");
            assert_float_eq(matrix.zx(), expected.zx, "zx field after the rejection");
            assert_float_eq(matrix.zy(), expected.zy, "zy field after the rejection");
            assert_float_eq(matrix.zz(), expected.zz, "zz field after the rejection");
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
            assert_float_eq(matrix.zz(), 2.0, "zz field after the rejection");
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
                spacecraft.inertia_matrix().0.m11,
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
        fn skip_field_setter_is_infallible() {
            let mut spacecraft = Spacecraft::new(VALID_SPACECRAFT_DRAFT)
                .expect("The valid spacecraft draft must build a spacecraft");

            spacecraft.set_primary_orbited_body(CelestialBodyKind::Sun);

            assert!(
                matches!(spacecraft.primary_orbited_body(), CelestialBodyKind::Sun),
                "The skipped field setter must store the new value without any validation"
            );
        }

        #[test]
        fn to_draft_from_draft_round_trip() {
            let matrix = InertiaMatrixSerializable::new(VALID_INERTIA_DRAFT)
                .expect("The valid inertia draft must build an inertia matrix");
            let restored_matrix = InertiaMatrixSerializable::from_draft(matrix.to_draft())
                .expect("The draft of a valid inertia matrix must validate again");
            assert_float_eq(
                restored_matrix.xx(),
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

    /// Serde wire format and deserialization firewall.
    #[cfg(feature = "std")]
    mod serde_integration {
        use super::{VALID_SPACECRAFT_DRAFT, assert_float_eq};
        use crate::{CelestialBodyKind, InertiaMatrix, ShadowFraction, Spacecraft};

        /// Build the JSON document of a spacecraft with the given masses and inertia `inertia_xx`,
        /// every other entry matching the standard valid spacecraft draft.
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

        /// Build the JSON document matching the standard valid spacecraft draft.
        fn valid_spacecraft_json_value() -> serde_json::Value {
            spacecraft_json_value(1000.0, 600.0, 300.0, 2.0)
        }

        /// Return the message of the failed deserialization `result`,
        /// `described_as` naming the document that had to be rejected.
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
                spacecraft.inertia_matrix().0.m11,
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
        fn spacecraft_serialize_deserialize_round_trip() {
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
                restored.inertia_matrix().0.m22,
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
        fn spacecraft_deserialize_rejects_invalid_nested_inertia() {
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
        }

        #[test]
        fn inertia_matrix_deserialize_currently_bypasses_validation() {
            // Pins a known and accepted validation hole, the serde firewall of
            // InertiaMatrixSerializable is commented out so an asymmetric document is accepted.
            // Closing the hole on purpose must break this test.
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
            let matrix = serde_json::from_str::<InertiaMatrix>(&document)
                .expect("An asymmetric inertia document currently deserializes without error");

            assert_float_eq(matrix.0.m12, 5.0, "xy entry of the asymmetric matrix");
        }
    }
}
