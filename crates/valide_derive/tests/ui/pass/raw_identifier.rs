//! A field whose name is a raw identifier, which every derived name spells without its prefix.

/// A token whose kind field needs a raw identifier.
#[derive(Clone, valide_derive::Validate, valide_derive::Patch)]
struct Token {
    /// Kind of the token.
    #[validate(range(0.0..=1.0))]
    r#type: f64,
}

fn main() {
    let mut token = Token::new(TokenDraft { r#type: 0.5 }).expect("0.5 is inside the range");
    assert!(
        (token.r#type() - 0.5).abs() < f64::EPSILON,
        "The getter must keep the raw identifier of the field"
    );

    assert_eq!(
        Token::new(TokenDraft { r#type: 2.0 }).err(),
        Some(TokenValidationError::OutOfRange {
            field: TokenField::Type,
            range: "[0.0, 1.0]",
        }),
        "The field enum variant must come from the name without its raw prefix"
    );

    assert!(
        TokenDraft { r#type: 0.75 }.validate_type().is_ok(),
        "The field validator must come from the name without its raw prefix"
    );

    assert!(
        token.set_type(0.25).is_ok(),
        "The setter must come from the name without its raw prefix"
    );
    assert!(
        (token.r#type() - 0.25).abs() < f64::EPSILON,
        "An accepted patch must commit the new value"
    );
}
