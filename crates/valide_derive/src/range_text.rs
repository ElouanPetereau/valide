//! Rendering of the validated ranges into the text of the `range` error field.
//!
//! The module holds one rule table per accepted grammar form, the sugared range expression and the bound pair.
//! The rendered text is a documented output format and not a verbatim stringification.

use quote::ToTokens;
use syn::{Expr, ExprRange, Lit, RangeLimits, UnOp};

/// Separator between the two rendered bounds of a range.
const BOUND_SEPARATOR: &str = ", ";
/// Text of a bound that the rules normalize to the negative infinity.
const NEGATIVE_INFINITY_TEXT: &str = "-inf";
/// Text of a bound that the rules normalize to the positive infinity.
const POSITIVE_INFINITY_TEXT: &str = "+inf";
/// Last path segment of a bound that reaches the positive infinity.
const INFINITY_CONSTANT: &str = "INFINITY";
/// Last path segment of a bound that reaches the negative infinity.
const NEGATIVE_INFINITY_CONSTANT: &str = "NEG_INFINITY";

/// One bound of the bound pair form of a range.
/// Two variants hold the expression of the bound value.
pub(crate) enum BoundKind<'expression> {
    /// An included bound.
    Included(&'expression Expr),
    /// An excluded bound.
    Excluded(&'expression Expr),
    /// A bound that reaches the infinity.
    Unbounded,
}

/// Render the range text of the sugared form from the given `range` expression.
/// The function normalizes the bounds like the bound pair form does, which the unit tests pin.
pub(crate) fn sugared_text(range: &ExprRange) -> String {
    // The check accepts the infinity of a missing bound, so its bracket is inclusive
    let lower_text = range.start.as_ref().map_or_else(
        || format!("[{NEGATIVE_INFINITY_TEXT}"),
        |start| format!("[{}", normalized_text(start)),
    );
    let upper_bracket = match range.limits {
        RangeLimits::Closed(_) => ']',
        RangeLimits::HalfOpen(_) => '[',
    };
    let upper_text = range.end.as_ref().map_or_else(
        || format!("{POSITIVE_INFINITY_TEXT}]"),
        |end| format!("{}{upper_bracket}", normalized_text(end)),
    );

    format!("{lower_text}{BOUND_SEPARATOR}{upper_text}")
}

/// Render the range text of the bound pair form from the given `lower` and `upper` bounds.
/// The function normalizes the bounds, which the unit tests pin.
pub(crate) fn bound_pair_text(lower: &BoundKind<'_>, upper: &BoundKind<'_>) -> String {
    // The check accepts the infinity of an unbounded end, so its bracket is inclusive
    let lower_text = match lower {
        BoundKind::Included(value) => format!("[{}", normalized_text(value)),
        BoundKind::Excluded(value) => format!("]{}", normalized_text(value)),
        BoundKind::Unbounded => format!("[{NEGATIVE_INFINITY_TEXT}"),
    };
    let upper_text = match upper {
        BoundKind::Included(value) => format!("{}]", normalized_text(value)),
        BoundKind::Excluded(value) => format!("{}[", normalized_text(value)),
        BoundKind::Unbounded => format!("{POSITIVE_INFINITY_TEXT}]"),
    };

    format!("{lower_text}{BOUND_SEPARATOR}{upper_text}")
}

/// Render the given `tokens` as text with every whitespace removed.
/// The token stream rendering inserts spaces that the range text must not carry.
fn token_text(tokens: &impl ToTokens) -> String {
    tokens
        .to_token_stream()
        .to_string()
        .split_whitespace()
        .collect()
}

/// Render the given bound `value` with the normalization rules of the bound pair form.
fn normalized_text(value: &Expr) -> String {
    if let Some(text) = literal_text(value) {
        return text;
    }
    if let Some(text) = infinity_text(value) {
        return text;
    }

    token_text(value)
}

/// Render the given `value` as a numeric literal without its type suffix, when it is one.
fn literal_text(value: &Expr) -> Option<String> {
    let Expr::Lit(literal) = value else {
        return None;
    };
    let (text, suffix) = match &literal.lit {
        Lit::Float(float) => (token_text(float), float.suffix()),
        Lit::Int(integer) => (token_text(integer), integer.suffix()),
        _ => return None,
    };
    if suffix.is_empty() {
        return Some(text);
    }
    let without_suffix = text.strip_suffix(suffix).unwrap_or(text.as_str());
    let without_separator = without_suffix.strip_suffix('_').unwrap_or(without_suffix);

    Some(without_separator.to_owned())
}

/// Render the given `value` as an infinity text, when it reaches an infinity.
fn infinity_text(value: &Expr) -> Option<String> {
    if let Expr::Unary(unary) = value {
        if matches!(unary.op, UnOp::Neg(_)) && ends_with_constant(&unary.expr, INFINITY_CONSTANT) {
            return Some(NEGATIVE_INFINITY_TEXT.to_owned());
        }

        return None;
    }
    if ends_with_constant(value, INFINITY_CONSTANT) {
        return Some(POSITIVE_INFINITY_TEXT.to_owned());
    }
    if ends_with_constant(value, NEGATIVE_INFINITY_CONSTANT) {
        return Some(NEGATIVE_INFINITY_TEXT.to_owned());
    }

    None
}

/// Whether the given `value` is a path whose last segment is the constant called `constant_name`.
fn ends_with_constant(value: &Expr, constant_name: &str) -> bool {
    let Expr::Path(path) = value else {
        return false;
    };
    let Some(last_segment) = path.path.segments.last() else {
        return false;
    };

    last_segment.ident == constant_name
}

#[cfg(test)]
mod tests {
    use syn::{Expr, ExprRange};

    use super::{BoundKind, bound_pair_text, sugared_text};

    /// Parse the given `source` as a range expression.
    fn range(source: &str) -> ExprRange {
        syn::parse_str(source).expect("the range source must parse as a range expression")
    }

    /// Parse the given `source` as an expression.
    fn expression(source: &str) -> Expr {
        syn::parse_str(source).expect("the source must parse as an expression")
    }

    #[test]
    fn sugared_inclusive_upper_bound() {
        assert_eq!(
            sugared_text(&range("0.0..=1.0")),
            "[0.0, 1.0]",
            "An inclusive upper bound must be rendered with a closing bracket"
        );
    }

    #[test]
    fn sugared_keeps_the_digit_separators() {
        assert_eq!(
            sugared_text(&range("0.0..=30_000.0")),
            "[0.0, 30_000.0]",
            "The digit separators of a bound literal must survive the rendering"
        );
        assert_eq!(
            sugared_text(&range("0.0..=10_000.0")),
            "[0.0, 10_000.0]",
            "The digit separators of a bound literal must survive the rendering"
        );
    }

    #[test]
    fn sugared_exclusive_upper_bound_normalizes_the_infinity() {
        assert_eq!(
            sugared_text(&range("0.0..f64::INFINITY")),
            "[0.0, +inf[",
            "The sugared form must normalize the infinity like the bound pair form does"
        );
    }

    #[test]
    fn sugared_missing_bounds() {
        assert_eq!(
            sugared_text(&range("0.0..")),
            "[0.0, +inf]",
            "A missing upper bound must be rendered as an included positive infinity"
        );
        assert_eq!(
            sugared_text(&range("..=1.0")),
            "[-inf, 1.0]",
            "A missing lower bound must be rendered as an included negative infinity"
        );
        assert_eq!(
            sugared_text(&range("..")),
            "[-inf, +inf]",
            "Two missing bounds must be rendered as the two included infinities"
        );
    }

    #[test]
    fn bound_pair_of_two_excluded_bounds() {
        let lower = expression("0.0_f64");
        let upper = expression("f64::INFINITY");

        assert_eq!(
            bound_pair_text(&BoundKind::Excluded(&lower), &BoundKind::Excluded(&upper)),
            "]0.0, +inf[",
            "Two excluded bounds must be rendered with outward brackets and a normalized infinity"
        );
    }

    #[test]
    fn bound_pair_of_two_included_bounds() {
        let lower = expression("0.0");
        let upper = expression("1.0");

        assert_eq!(
            bound_pair_text(&BoundKind::Included(&lower), &BoundKind::Included(&upper)),
            "[0.0, 1.0]",
            "Two included bounds must be rendered with inward brackets"
        );
    }

    #[test]
    fn bound_pair_of_two_unbounded_ends() {
        assert_eq!(
            bound_pair_text(&BoundKind::Unbounded, &BoundKind::Unbounded),
            "[-inf, +inf]",
            "Two unbounded ends must be rendered as the two included infinities"
        );
    }

    #[test]
    fn bound_pair_normalizes_the_negative_infinity() {
        let named_constant = expression("f64::NEG_INFINITY");
        let negated_constant = expression("-f64::INFINITY");

        assert_eq!(
            bound_pair_text(
                &BoundKind::Included(&named_constant),
                &BoundKind::Excluded(&negated_constant)
            ),
            "[-inf, -inf[",
            "Both spellings of the negative infinity must be normalized"
        );
    }

    #[test]
    fn bound_pair_strips_the_literal_type_suffix() {
        let lower = expression("30_000.0_f64");
        let upper = expression("40_000.0f64");

        assert_eq!(
            bound_pair_text(&BoundKind::Included(&lower), &BoundKind::Included(&upper)),
            "[30_000.0, 40_000.0]",
            "The type suffix must be stripped with the separator preceding it"
        );
    }
}
