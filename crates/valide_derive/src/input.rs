//! Parsing of a derive input into the intermediate representation.
//!
//! The module owns the whole attribute grammar, from the field `validate` markers to the repeatable `final_validation` and the `draft_attr` passthrough.
//! It produces every diagnostic of the two derives.
//! The diagnostics accumulate and point at the offending tokens.

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens as _, quote};
use syn::{
    Attribute, Data, DeriveInput, Error, Expr, Field, Fields, Ident, Index, Member, Meta, MetaList,
    Path, Result, Token, parse::ParseStream, punctuated::Punctuated, spanned::Spanned as _,
};

use crate::{
    intermediate_representation::{
        FieldIntermediateRepresentation, FinalValidation, Rule, Shape,
        TypeIntermediateRepresentation,
    },
    naming,
    range_text::{self, BoundKind},
};

/// Name of the marker attribute of a field.
const VALIDATE_ATTRIBUTE: &str = "validate";
/// Name of the attribute that declares a final validation.
const FINAL_VALIDATION_ATTRIBUTE: &str = "final_validation";
/// Name of the attribute whose payload the generated draft carries verbatim.
const DRAFT_ATTR_ATTRIBUTE: &str = "draft_attr";
/// Name of the documentation attribute that the draft fields also carry.
const DOC_ATTRIBUTE: &str = "doc";
/// Name of the serde attribute that carries the deserialization validation.
const SERDE_ATTRIBUTE: &str = "serde";
/// Key of the serde validation that triggers the serde derives on the draft.
const TRY_FROM_KEY: &str = "try_from";
/// Key that introduces the error type of a final validation.
const ERROR_KEY: &str = "error";
/// Marker that requires a value inside a range.
const RANGE_MARKER: &str = "range";
/// Marker that requires a finite number.
const FINITE_MARKER: &str = "finite";
/// Marker that delegates the validation to the type of the field.
const NESTED_MARKER: &str = "nested";
/// Marker that excludes the field from every validation.
const SKIP_MARKER: &str = "skip";
/// Name of the `Bound` variant of an included bound.
const INCLUDED_BOUND: &str = "Included";
/// Name of the `Bound` variant of an excluded bound.
const EXCLUDED_BOUND: &str = "Excluded";
/// Name of the `Bound` variant of an unbounded end.
const UNBOUNDED_BOUND: &str = "Unbounded";
/// Logical name of the single field of a newtype.
const NEWTYPE_LOGICAL_NAME: &str = "value";
/// Message that describes the markers of a field.
const MARKER_MESSAGE: &str =
    "a field needs exactly one #[validate(...)] marker among range(...), finite, nested and skip";
/// Message that describes the arguments of a `range` marker.
const RANGE_MESSAGE: &str =
    "#[validate(range(...))] takes one range expression or two core::ops::Bound values";
/// Message that describes the accepted bounds of the bound pair form.
const BOUND_MESSAGE: &str =
    "a bound must be Bound::Included(value), Bound::Excluded(value) or Bound::Unbounded";
/// Message that describes the arguments of a `final_validation` attribute.
const FINAL_VALIDATION_MESSAGE: &str =
    "#[final_validation(...)] must be written #[final_validation(fn, error = Type)]";
/// Message that describes the payload of a `draft_attr` attribute.
const DRAFT_ATTR_MESSAGE: &str =
    "#[draft_attr(...)] takes the attribute to re-emit on the generated draft as its payload";

/// Parse the given `derive_input` into the intermediate representation of a validated type.
/// Return every grammar error found. A single compilation reports all of them.
pub(crate) fn parse(derive_input: &DeriveInput) -> Result<TypeIntermediateRepresentation> {
    require_supported_type(derive_input)?;
    let (shape, raw_fields) = struct_fields(derive_input)?;

    let mut errors: Vec<Error> = Vec::new();
    let mut fields: Vec<FieldIntermediateRepresentation> = Vec::new();
    for (position, field) in raw_fields.into_iter().enumerate() {
        match parse_field(field, position) {
            Ok(field_ir) => fields.push(field_ir),
            Err(error) => errors.push(error),
        }
    }

    let mut final_validations: Vec<FinalValidation> = Vec::new();
    let mut draft_passthrough: Vec<TokenStream> = Vec::new();
    for attribute in &derive_input.attrs {
        if attribute.path().is_ident(FINAL_VALIDATION_ATTRIBUTE) {
            match parse_final_validation(attribute) {
                Ok(final_validation) => final_validations.push(final_validation),
                Err(error) => errors.push(error),
            }
        } else if attribute.path().is_ident(DRAFT_ATTR_ATTRIBUTE) {
            match draft_attr_payload(attribute) {
                Ok(payload) => draft_passthrough.push(payload),
                Err(error) => errors.push(error),
            }
        }
    }

    if let Err(error) = check_wrapper_variants(&fields, &final_validations) {
        errors.push(error);
    }
    if let Err(error) = check_field_variants(&fields) {
        errors.push(error);
    }
    accumulated(errors)?;

    let ident = derive_input.ident.clone();
    let draft_ident = naming::suffixed_ident(&ident, naming::DRAFT_SUFFIX);
    let field_enum_ident = naming::suffixed_ident(&ident, naming::FIELD_ENUM_SUFFIX);
    let error_ident = naming::suffixed_ident(&ident, naming::VALIDATION_ERROR_SUFFIX);

    Ok(TypeIntermediateRepresentation {
        ident,
        vis: derive_input.vis.clone(),
        draft_ident,
        field_enum_ident,
        error_ident,
        shape,
        fields,
        final_validations,
        emit_draft_serde: has_serde_try_from(&derive_input.attrs),
        draft_passthrough,
    })
}

/// Return the given `errors` combined into a single error, or `Ok(())` when there is none.
fn accumulated(errors: Vec<Error>) -> Result<()> {
    let mut errors = errors.into_iter();
    let Some(mut combined) = errors.next() else {
        return Ok(());
    };
    for error in errors {
        combined.combine(error);
    }

    Err(combined)
}

/// Check that the given `derive_input` is a type the derives support.
fn require_supported_type(derive_input: &DeriveInput) -> Result<()> {
    if !derive_input.generics.params.is_empty() || derive_input.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &derive_input.generics,
            "a validated type must not be generic",
        ));
    }
    for attribute in &derive_input.attrs {
        if attribute.path().is_ident(VALIDATE_ATTRIBUTE) {
            return Err(Error::new_spanned(
                attribute,
                "#[validate(...)] describes a field, a whole type is described by \
                 #[final_validation(...)]",
            ));
        }
    }

    Ok(())
}

/// Return the shape of the given `derive_input` with its fields in declaration order.
fn struct_fields(derive_input: &DeriveInput) -> Result<(Shape, Vec<&Field>)> {
    let data_struct = match &derive_input.data {
        Data::Struct(data_struct) => data_struct,
        Data::Enum(data_enum) => {
            return Err(Error::new_spanned(
                data_enum.enum_token,
                "a validated type must be a struct, not an enum",
            ));
        }
        Data::Union(data_union) => {
            return Err(Error::new_spanned(
                data_union.union_token,
                "a validated type must be a struct, not a union",
            ));
        }
    };

    match &data_struct.fields {
        Fields::Named(named_fields) => Ok((Shape::Named, named_fields.named.iter().collect())),
        Fields::Unnamed(unnamed_fields) => {
            let mut unnamed = unnamed_fields.unnamed.iter();
            match (unnamed.next(), unnamed.next()) {
                (Some(field), None) => Ok((Shape::Newtype, vec![field])),
                _ => Err(Error::new_spanned(
                    &unnamed_fields.unnamed,
                    "a validated tuple struct must have exactly one field",
                )),
            }
        }
        Fields::Unit => Err(Error::new_spanned(
            &derive_input.ident,
            "a validated type must have at least one field",
        )),
    }
}

/// Parse the given `field`, at `position` in the declaration order, into its representation.
fn parse_field(field: &Field, position: usize) -> Result<FieldIntermediateRepresentation> {
    let (member, logical_name, name_span) = field.ident.as_ref().map_or_else(
        || {
            (
                Member::Unnamed(Index::from(position)),
                NEWTYPE_LOGICAL_NAME.to_owned(),
                field.ty.span(),
            )
        },
        |ident| {
            let name = ident.to_string();

            (
                Member::Named(ident.clone()),
                naming::plain_name(&name).to_owned(),
                ident.span(),
            )
        },
    );

    let mut docs: Vec<Attribute> = Vec::new();
    let mut passthrough: Vec<TokenStream> = Vec::new();
    let mut markers: Vec<&Attribute> = Vec::new();
    for attribute in &field.attrs {
        let path = attribute.path();
        if path.is_ident(DOC_ATTRIBUTE) {
            docs.push(attribute.clone());
        } else if path.is_ident(VALIDATE_ATTRIBUTE) {
            markers.push(attribute);
        } else if path.is_ident(SERDE_ATTRIBUTE) {
            // The draft carries the wire format of the type behind a deserialization validation,
            // so the serde attributes of a field must reach the draft field
            passthrough.push(attribute.meta.to_token_stream());
        } else if path.is_ident(DRAFT_ATTR_ATTRIBUTE) {
            passthrough.push(draft_attr_payload(attribute)?);
        } else if path.is_ident(FINAL_VALIDATION_ATTRIBUTE) {
            return Err(Error::new_spanned(
                attribute,
                "#[final_validation(...)] describes a whole type, not a field",
            ));
        }
    }

    let mut markers = markers.into_iter();
    let Some(marker) = markers.next() else {
        return Err(Error::new(name_span, MARKER_MESSAGE));
    };
    if let Some(extra_marker) = markers.next() {
        return Err(Error::new_spanned(extra_marker, MARKER_MESSAGE));
    }

    // The validator and the setter of every field derive their names from the logical name,
    // so the check runs before the rule tells whether the field also carries a variant
    let field_variant = naming::field_variant(&logical_name, name_span)?;
    let rule = parse_marker(marker, &logical_name, name_span)?;
    // Only a range or a finite field can name itself inside an error,
    // so only those two rules get a field enum variant
    let variant = match &rule {
        Rule::Range { .. } | Rule::Finite => Some(field_variant),
        Rule::Nested { .. } | Rule::Skip => None,
    };

    Ok(FieldIntermediateRepresentation {
        member,
        logical_name,
        variant,
        ty: field.ty.clone(),
        docs,
        passthrough,
        rule,
    })
}

/// Parse the single marker of the given `validate` `attribute` into the rule of a field.
/// The field is called `logical_name`, with the name span `name_span`.
fn parse_marker(attribute: &Attribute, logical_name: &str, name_span: Span) -> Result<Rule> {
    let markers = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
    let mut markers = markers.into_iter();
    let Some(marker) = markers.next() else {
        return Err(Error::new_spanned(attribute, MARKER_MESSAGE));
    };
    if let Some(extra_marker) = markers.next() {
        return Err(Error::new_spanned(extra_marker, MARKER_MESSAGE));
    }

    if let Meta::List(list) = &marker {
        if list.path.is_ident(RANGE_MARKER) {
            return parse_range(list);
        }

        return Err(Error::new_spanned(&list.path, MARKER_MESSAGE));
    }
    let Meta::Path(path) = &marker else {
        return Err(Error::new_spanned(&marker, MARKER_MESSAGE));
    };
    if path.is_ident(FINITE_MARKER) {
        return Ok(Rule::Finite);
    }
    if path.is_ident(NESTED_MARKER) {
        return Ok(Rule::Nested {
            wrapper_variant: naming::nested_wrapper_variant(logical_name, name_span)?,
        });
    }
    if path.is_ident(SKIP_MARKER) {
        return Ok(Rule::Skip);
    }

    Err(Error::new_spanned(path, MARKER_MESSAGE))
}

/// Parse the arguments of the given `range` marker `list` into the rule of a field.
fn parse_range(list: &MetaList) -> Result<Rule> {
    let arguments = list.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?;
    let mut arguments = arguments.iter();
    let bounds = (arguments.next(), arguments.next(), arguments.next());

    if let (Some(single), None, None) = bounds {
        let Expr::Range(range) = single else {
            return Err(Error::new_spanned(single, RANGE_MESSAGE));
        };

        return Ok(Rule::Range {
            check_tokens: range.to_token_stream(),
            text: range_text::sugared_text(range),
        });
    }
    if let (Some(lower), Some(upper), None) = bounds {
        let lower_bound = parse_bound(lower)?;
        let upper_bound = parse_bound(upper)?;

        return Ok(Rule::Range {
            check_tokens: quote! { (#lower, #upper) },
            text: range_text::bound_pair_text(&lower_bound, &upper_bound),
        });
    }

    Err(Error::new_spanned(list, RANGE_MESSAGE))
}

/// Classify the given `expression` as one bound of the bound pair form of a range.
/// The last segment of the path carries the meaning, so the parser accepts every spelling of `Bound`.
fn parse_bound(expression: &Expr) -> Result<BoundKind<'_>> {
    if let Expr::Path(path_expression) = expression {
        if last_segment_is(&path_expression.path, UNBOUNDED_BOUND) {
            return Ok(BoundKind::Unbounded);
        }

        return Err(Error::new_spanned(expression, BOUND_MESSAGE));
    }

    let Expr::Call(call) = expression else {
        return Err(Error::new_spanned(expression, BOUND_MESSAGE));
    };
    let Expr::Path(function) = call.func.as_ref() else {
        return Err(Error::new_spanned(&call.func, BOUND_MESSAGE));
    };
    let mut call_arguments = call.args.iter();
    let (Some(value), None) = (call_arguments.next(), call_arguments.next()) else {
        return Err(Error::new_spanned(call, BOUND_MESSAGE));
    };
    if last_segment_is(&function.path, INCLUDED_BOUND) {
        return Ok(BoundKind::Included(value));
    }
    if last_segment_is(&function.path, EXCLUDED_BOUND) {
        return Ok(BoundKind::Excluded(value));
    }

    Err(Error::new_spanned(&call.func, BOUND_MESSAGE))
}

/// Whether the last segment of the given `path` is called `segment_name`.
fn last_segment_is(path: &Path, segment_name: &str) -> bool {
    let Some(last_segment) = path.segments.last() else {
        return false;
    };

    last_segment.ident == segment_name
}

/// Parse the given `final_validation` `attribute` into its representation.
fn parse_final_validation(attribute: &Attribute) -> Result<FinalValidation> {
    let (fn_ident, error_ty) = attribute.parse_args_with(final_validation_arguments)?;
    let wrapper_variant =
        naming::final_validation_wrapper_variant(&fn_ident.to_string(), fn_ident.span())?;

    Ok(FinalValidation {
        fn_ident,
        error_ty,
        wrapper_variant,
    })
}

/// Parse the arguments of a `final_validation` attribute from `input`.
/// The arguments are the validation function and its error type.
fn final_validation_arguments(input: ParseStream<'_>) -> Result<(Ident, Path)> {
    let function: Ident = input.parse()?;
    if input.is_empty() {
        return Err(Error::new(function.span(), FINAL_VALIDATION_MESSAGE));
    }
    let _comma: Token![,] = input.parse()?;
    let key: Ident = input.parse()?;
    if key != ERROR_KEY {
        return Err(Error::new_spanned(&key, FINAL_VALIDATION_MESSAGE));
    }
    let _assign: Token![=] = input.parse()?;
    let error_type: Path = input.parse()?;
    if !input.is_empty() {
        return Err(Error::new(input.span(), FINAL_VALIDATION_MESSAGE));
    }

    Ok((function, error_type))
}

/// Return the payload of the given `draft_attr` `attribute`.
fn draft_attr_payload(attribute: &Attribute) -> Result<TokenStream> {
    let Meta::List(list) = &attribute.meta else {
        return Err(Error::new_spanned(attribute, DRAFT_ATTR_MESSAGE));
    };

    Ok(list.tokens.clone())
}

/// Whether the given `attributes` carry a surviving serde deserialization validation.
/// The generated draft mirrors the validation with its own serde derives.
fn has_serde_try_from(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        if !attribute.path().is_ident(SERDE_ATTRIBUTE) {
            return false;
        }

        let mut carries_firewall = false;
        // A serde attribute that the walk cannot read carries no key at all, and the draft then mirrors nothing
        let _walk: Result<()> = attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident(TRY_FROM_KEY) {
                carries_firewall = true;
            }
            // The walk owns the value of the key, which ends at the next key of the attribute
            while !meta.input.is_empty() && !meta.input.peek(Token![,]) {
                let _value: TokenTree = meta.input.parse()?;
            }

            Ok(())
        });

        carries_firewall
    })
}

/// Check that the field enum variants generated for `fields` are all distinct.
fn check_field_variants(fields: &[FieldIntermediateRepresentation]) -> Result<()> {
    let variants: Vec<Ident> = fields
        .iter()
        .filter_map(|field| field.variant.clone())
        .collect();

    let Some((first, second)) = naming::first_collision(&variants) else {
        return Ok(());
    };
    let mut error = Error::new(
        second.span(),
        format!("the generated field variant `{second}` would be generated twice"),
    );
    error.combine(Error::new(
        first.span(),
        format!("`{first}` is already generated here"),
    ));

    Err(error)
}

/// Check that the wrapper variants generated for `fields` and `final_validations` are all distinct.
fn check_wrapper_variants(
    fields: &[FieldIntermediateRepresentation],
    final_validations: &[FinalValidation],
) -> Result<()> {
    let mut variants: Vec<Ident> = Vec::new();
    for field in fields {
        if let Rule::Nested { wrapper_variant } = &field.rule {
            variants.push(wrapper_variant.clone());
        }
    }
    for final_validation in final_validations {
        variants.push(final_validation.wrapper_variant.clone());
    }

    let Some((first, second)) = naming::first_collision(&variants) else {
        return Ok(());
    };
    let mut error = Error::new(
        second.span(),
        format!("the generated error variant `{second}` would be generated twice"),
    );
    // A field and a final validation reach this check in their own order,
    // so the note claims no order between the two spans
    error.combine(Error::new(
        first.span(),
        format!("the same `{first}` variant also comes from here"),
    ));

    Err(error)
}
