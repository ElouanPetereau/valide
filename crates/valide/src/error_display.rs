//! Rendering of the generated validation errors.
//!
//! The macro generated code calls these helpers from the `Display` of an error, so they stay `core` only.

use core::{
    fmt::{Debug, Formatter, Result as FormatResult},
    ops::Bound,
};

/// Text of a bound that reaches the negative infinity.
const NEGATIVE_INFINITY_TEXT: &str = "-inf";
/// Text of a bound that reaches the positive infinity.
const POSITIVE_INFINITY_TEXT: &str = "+inf";
/// Separator between the two written bounds of a range.
const BOUND_SEPARATOR: &str = ", ";

/// Write the range of the given `lower` and `upper` bounds in `formatter`.
///
/// An included bound gets an inward bracket, an excluded bound gets an outward one and
/// an unbounded end gets the infinity it reaches with an inward bracket.
#[doc(hidden)]
pub fn write_range<Value: Debug>(
    formatter: &mut Formatter<'_>,
    lower: &Bound<Value>,
    upper: &Bound<Value>,
) -> FormatResult {
    // The check accepts the infinity that an unbounded end reaches, so its bracket is the inward one
    match lower {
        Bound::Included(value) => write!(formatter, "[{value:?}")?,
        Bound::Excluded(value) => write!(formatter, "]{value:?}")?,
        Bound::Unbounded => write!(formatter, "[{NEGATIVE_INFINITY_TEXT}")?,
    }
    formatter.write_str(BOUND_SEPARATOR)?;
    match upper {
        Bound::Included(value) => write!(formatter, "{value:?}]"),
        Bound::Excluded(value) => write!(formatter, "{value:?}["),
        Bound::Unbounded => write!(formatter, "{POSITIVE_INFINITY_TEXT}]"),
    }
}

#[cfg(test)]
mod tests {
    use core::{
        fmt::{Display, Formatter, Result as FormatResult},
        ops::Bound,
    };

    use crate::error_display::write_range;

    /// Range whose [`Display`] writes its two bounds through the helper of the crate.
    struct WrittenRange {
        /// Lower bound of the range.
        lower: Bound<f64>,
        /// Upper bound of the range.
        upper: Bound<f64>,
    }

    impl Display for WrittenRange {
        // The parameter keeps the name of the trait, which the generated code also keeps
        fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
            write_range(f, &self.lower, &self.upper)
        }
    }

    /// Return the text that the helper writes for the given `lower` and `upper` bounds.
    fn written_range(lower: Bound<f64>, upper: Bound<f64>) -> String {
        WrittenRange { lower, upper }.to_string()
    }

    #[test]
    fn every_bound_kind_of_the_lower_end() {
        assert_eq!(
            written_range(Bound::Included(0.0), Bound::Included(1.0)),
            "[0.0, 1.0]",
            "An included lower bound must get the inward bracket"
        );
        assert_eq!(
            written_range(Bound::Excluded(0.0), Bound::Included(1.0)),
            "]0.0, 1.0]",
            "An excluded lower bound must get the outward bracket"
        );
        assert_eq!(
            written_range(Bound::Unbounded, Bound::Included(1.0)),
            "[-inf, 1.0]",
            "An unbounded lower end must reach the negative infinity with the inward bracket"
        );
    }

    #[test]
    fn every_bound_kind_of_the_upper_end() {
        assert_eq!(
            written_range(Bound::Included(0.0), Bound::Included(1.0)),
            "[0.0, 1.0]",
            "An included upper bound must get the inward bracket"
        );
        assert_eq!(
            written_range(Bound::Included(0.0), Bound::Excluded(1.0)),
            "[0.0, 1.0[",
            "An excluded upper bound must get the outward bracket"
        );
        assert_eq!(
            written_range(Bound::Included(0.0), Bound::Unbounded),
            "[0.0, +inf]",
            "An unbounded upper end must reach the positive infinity with the inward bracket"
        );
        assert_eq!(
            written_range(Bound::Unbounded, Bound::Unbounded),
            "[-inf, +inf]",
            "Two unbounded ends must reach the two infinities"
        );
    }

    #[test]
    fn a_bound_value_comes_from_its_debug() {
        assert_eq!(
            written_range(Bound::Excluded(10_000.0), Bound::Excluded(f64::INFINITY)),
            "]10000.0, inf[",
            "A bound value must come as its Debug writes it, without a digit separator"
        );
    }
}
