//! Parsing of a derive input into the intermediate representation.
//!
//! The module owns the whole attribute grammar, from the field `validate` markers to the repeatable `final_validation` and the `draft_attr` passthrough.
//! It produces every diagnostic of the two derives.
//! The diagnostics accumulate and point at the offending tokens.

use core::ptr;

use proc_macro2::{Spacing, Span, TokenStream, TokenTree};
use quote::{ToTokens as _, quote};
use syn::{
    Attribute, Data, DataStruct, DeriveInput, Error, Expr, Field, Fields, GenericParam, Generics,
    Ident, Index, Member, Meta, MetaList, Path, Result, Token, Variant, parse::ParseStream,
    punctuated::Punctuated, spanned::Spanned as _,
};

use crate::{
    intermediate_representation::{
        FieldIntermediateRepresentation, FieldRule, FinalValidation, Shape,
        TypeIntermediateRepresentation, VariantIntermediateRepresentation, VariantKind,
        VariantRule,
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
/// Message that describes the markers of a variant payload.
const PAYLOAD_MARKER_MESSAGE: &str =
    "a variant payload needs exactly one #[validate(...)] marker among nested and skip";
/// Message of the rejection of a marker that sits on a variant instead of the payload of the variant.
const VARIANT_MARKER_MESSAGE: &str =
    "#[validate(...)] marks the payload of a variant, not the variant itself";
/// Message that describes the accepted shapes of a variant.
const VARIANT_SHAPE_MESSAGE: &str =
    "a validated variant must be a unit variant or a tuple variant with exactly one payload";
/// Message of the rejection of a union.
const UNION_MESSAGE: &str = "a validated type must be a struct or an enum, not a union";
/// Message of the rejection of a `final_validation` attribute on a field.
const FIELD_FINAL_VALIDATION_MESSAGE: &str =
    "#[final_validation(...)] describes a whole type, not a field";
/// Message of the rejection of a `final_validation` attribute on a variant.
const VARIANT_FINAL_VALIDATION_MESSAGE: &str =
    "#[final_validation(...)] describes a whole type, not a variant";
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
/// Message of the rejection of a generic parameter that the generated error enum cannot carry.
const ERROR_PAYLOAD_SUBSET_MESSAGE: &str = "the generated error enum must carry every generic parameter or none, and this parameter appears in no nested field type and in no final validation error type";

/// Parse the given `derive_input` into the intermediate representation of a validated type.
/// Return every grammar error found. A single compilation reports all of them.
pub(crate) fn parse(derive_input: &DeriveInput) -> Result<TypeIntermediateRepresentation> {
    require_supported_type(derive_input)?;

    let mut errors: Vec<Error> = Vec::new();
    let mut fields: Vec<FieldIntermediateRepresentation> = Vec::new();
    let mut variants: Vec<VariantIntermediateRepresentation> = Vec::new();
    let shape = match &derive_input.data {
        Data::Struct(data_struct) => {
            let (shape, raw_fields) = struct_fields(&derive_input.ident, data_struct)?;
            for (position, field) in raw_fields.into_iter().enumerate() {
                match parse_field(field, position) {
                    Ok(field_ir) => fields.push(field_ir),
                    Err(error) => errors.push(error),
                }
            }

            shape
        }
        Data::Enum(data_enum) => {
            for variant in &data_enum.variants {
                match parse_variant(variant) {
                    Ok(variant_ir) => variants.push(variant_ir),
                    Err(error) => errors.push(error),
                }
            }

            Shape::Enum
        }
        Data::Union(data_union) => {
            return Err(Error::new_spanned(data_union.union_token, UNION_MESSAGE));
        }
    };

    let mut final_validations: Vec<FinalValidation> = Vec::new();
    let mut draft_passthrough: Vec<TokenStream> = Vec::new();
    for attribute in &derive_input.attrs {
        if attribute.path().is_ident(FINAL_VALIDATION_ATTRIBUTE) {
            match parse_final_validation(attribute) {
                Ok(final_validation) => final_validations.push(final_validation),
                Err(error) => errors.push(error),
            }
        } else if attribute.path().is_ident(DRAFT_ATTR_ATTRIBUTE) {
            match draft_attr_tokens(attribute) {
                Ok(payload) => draft_passthrough.push(payload),
                Err(error) => errors.push(error),
            }
        }
    }

    if let Err(error) = check_wrapper_variants(&fields, &variants, &final_validations) {
        errors.push(error);
    }
    if let Err(error) = check_field_enum_variants(&fields) {
        errors.push(error);
    }
    let used_parameters = used_error_parameters(
        &derive_input.generics,
        &fields,
        &variants,
        &final_validations,
    );
    if let Err(error) = check_error_parameters(&derive_input.generics, &used_parameters) {
        errors.push(error);
    }
    accumulated(errors)?;

    let ident = derive_input.ident.clone();
    let generics = derive_input.generics.clone();
    let draft_ident = naming::suffixed_ident(&ident, naming::DRAFT_SUFFIX);
    let field_enum_ident = naming::suffixed_ident(&ident, naming::FIELD_ENUM_SUFFIX);
    let error_ident = naming::suffixed_ident(&ident, naming::VALIDATION_ERROR_SUFFIX);
    // The check above rejected every proper subset,
    // so a single parameter that reaches an error payload means that they all reach one
    let error_enum_is_generic = !used_parameters.is_empty();

    Ok(TypeIntermediateRepresentation {
        ident,
        vis: derive_input.vis.clone(),
        generics,
        draft_ident,
        field_enum_ident,
        error_ident,
        error_enum_is_generic,
        shape,
        fields,
        variants,
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

/// Return the shape of the given `data_struct` with its fields in declaration order.
/// The struct is called `type_ident`, which carries the diagnostic of a struct without a field.
fn struct_fields<'data>(
    type_ident: &Ident,
    data_struct: &'data DataStruct,
) -> Result<(Shape, Vec<&'data Field>)> {
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
            type_ident,
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
            passthrough.push(draft_attr_tokens(attribute)?);
        } else if path.is_ident(FINAL_VALIDATION_ATTRIBUTE) {
            return Err(Error::new_spanned(
                attribute,
                FIELD_FINAL_VALIDATION_MESSAGE,
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
    let rule = parse_field_marker(marker, &logical_name, name_span)?;
    // Only a range or a finite field can name itself inside an error,
    // so only those two rules get a field enum variant
    let variant = match &rule {
        FieldRule::Range { .. } | FieldRule::Finite => Some(field_variant),
        FieldRule::Nested { .. } | FieldRule::Skip => None,
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
fn parse_field_marker(
    attribute: &Attribute,
    logical_name: &str,
    name_span: Span,
) -> Result<FieldRule> {
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
            return parse_field_range(list);
        }

        return Err(Error::new_spanned(&list.path, MARKER_MESSAGE));
    }
    let Meta::Path(path) = &marker else {
        return Err(Error::new_spanned(&marker, MARKER_MESSAGE));
    };
    if path.is_ident(FINITE_MARKER) {
        return Ok(FieldRule::Finite);
    }
    if path.is_ident(NESTED_MARKER) {
        return Ok(FieldRule::Nested {
            wrapper_variant: naming::nested_wrapper_variant(logical_name, name_span)?,
        });
    }
    if path.is_ident(SKIP_MARKER) {
        return Ok(FieldRule::Skip);
    }

    Err(Error::new_spanned(path, MARKER_MESSAGE))
}

/// Parse the arguments of the given `range` marker `list` into the rule of a field.
fn parse_field_range(list: &MetaList) -> Result<FieldRule> {
    let arguments = list.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?;
    let mut arguments = arguments.iter();
    let bounds = (arguments.next(), arguments.next(), arguments.next());

    if let (Some(single), None, None) = bounds {
        let Expr::Range(range) = single else {
            return Err(Error::new_spanned(single, RANGE_MESSAGE));
        };

        return Ok(FieldRule::Range {
            check_tokens: range.to_token_stream(),
            text: range_text::sugared_text(range),
        });
    }
    if let (Some(lower), Some(upper), None) = bounds {
        let lower_bound = parse_bound(lower)?;
        let upper_bound = parse_bound(upper)?;

        return Ok(FieldRule::Range {
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

/// Parse the given `variant` of a validated enum into its representation.
/// The attributes of the variant reach the draft variant, and a marker belongs to the payload only.
fn parse_variant(variant: &Variant) -> Result<VariantIntermediateRepresentation> {
    let mut docs: Vec<Attribute> = Vec::new();
    let mut passthrough: Vec<TokenStream> = Vec::new();
    for attribute in &variant.attrs {
        let path = attribute.path();
        if path.is_ident(DOC_ATTRIBUTE) {
            docs.push(attribute.clone());
        } else if path.is_ident(SERDE_ATTRIBUTE) {
            // The draft carries the wire format of the enum behind a deserialization validation,
            // so the serde attributes of a variant must reach the draft variant
            passthrough.push(attribute.meta.to_token_stream());
        } else if path.is_ident(DRAFT_ATTR_ATTRIBUTE) {
            passthrough.push(draft_attr_tokens(attribute)?);
        } else if path.is_ident(VALIDATE_ATTRIBUTE) {
            return Err(Error::new_spanned(attribute, VARIANT_MARKER_MESSAGE));
        } else if path.is_ident(FINAL_VALIDATION_ATTRIBUTE) {
            return Err(Error::new_spanned(
                attribute,
                VARIANT_FINAL_VALIDATION_MESSAGE,
            ));
        }
    }
    let kind = parse_variant_kind(variant)?;

    Ok(VariantIntermediateRepresentation {
        ident: variant.ident.clone(),
        docs,
        passthrough,
        kind,
    })
}

/// Parse the fields of the given `variant` into the content of the variant.
/// A unit variant carries nothing and a tuple variant carries exactly one payload.
fn parse_variant_kind(variant: &Variant) -> Result<VariantKind> {
    let unnamed_fields = match &variant.fields {
        Fields::Unit => return Ok(VariantKind::Unit),
        Fields::Unnamed(unnamed_fields) => unnamed_fields,
        Fields::Named(named_fields) => {
            return Err(Error::new_spanned(named_fields, VARIANT_SHAPE_MESSAGE));
        }
    };
    let mut unnamed = unnamed_fields.unnamed.iter();
    let (Some(payload), None) = (unnamed.next(), unnamed.next()) else {
        return Err(Error::new_spanned(unnamed_fields, VARIANT_SHAPE_MESSAGE));
    };

    parse_payload(payload, &variant.ident)
}

/// Parse the single `payload` of the tuple variant called `variant_ident` into the content of the variant.
/// The name of the variant builds the wrapper variant of a nested payload.
fn parse_payload(payload: &Field, variant_ident: &Ident) -> Result<VariantKind> {
    let mut docs: Vec<Attribute> = Vec::new();
    let mut passthrough: Vec<TokenStream> = Vec::new();
    let mut markers: Vec<&Attribute> = Vec::new();
    for attribute in &payload.attrs {
        let path = attribute.path();
        if path.is_ident(DOC_ATTRIBUTE) {
            docs.push(attribute.clone());
        } else if path.is_ident(VALIDATE_ATTRIBUTE) {
            markers.push(attribute);
        } else if path.is_ident(SERDE_ATTRIBUTE) {
            passthrough.push(attribute.meta.to_token_stream());
        } else if path.is_ident(DRAFT_ATTR_ATTRIBUTE) {
            passthrough.push(draft_attr_tokens(attribute)?);
        } else if path.is_ident(FINAL_VALIDATION_ATTRIBUTE) {
            return Err(Error::new_spanned(
                attribute,
                FIELD_FINAL_VALIDATION_MESSAGE,
            ));
        }
    }

    let mut markers = markers.into_iter();
    let Some(marker) = markers.next() else {
        return Err(Error::new(payload.ty.span(), PAYLOAD_MARKER_MESSAGE));
    };
    if let Some(extra_marker) = markers.next() {
        return Err(Error::new_spanned(extra_marker, PAYLOAD_MARKER_MESSAGE));
    }
    let rule = parse_payload_marker(marker, variant_ident)?;

    Ok(VariantKind::Payload {
        ty: payload.ty.clone(),
        rule,
        docs,
        passthrough,
    })
}

/// Parse the single marker of the given `validate` `attribute` into the rule of a variant payload.
/// The payload belongs to the variant called `variant_ident`.
/// A payload accepts the two markers that delegate a rule, because the enum declares no rule of its own.
fn parse_payload_marker(attribute: &Attribute, variant_ident: &Ident) -> Result<VariantRule> {
    let markers = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
    let mut markers = markers.into_iter();
    let Some(marker) = markers.next() else {
        return Err(Error::new_spanned(attribute, PAYLOAD_MARKER_MESSAGE));
    };
    if let Some(extra_marker) = markers.next() {
        return Err(Error::new_spanned(extra_marker, PAYLOAD_MARKER_MESSAGE));
    }
    let Meta::Path(path) = &marker else {
        return Err(Error::new_spanned(&marker, PAYLOAD_MARKER_MESSAGE));
    };
    if path.is_ident(NESTED_MARKER) {
        return Ok(VariantRule::Nested {
            wrapper_variant: naming::nested_wrapper_variant(
                &variant_ident.to_string(),
                variant_ident.span(),
            )?,
        });
    }
    if path.is_ident(SKIP_MARKER) {
        return Ok(VariantRule::Skip);
    }

    Err(Error::new_spanned(path, PAYLOAD_MARKER_MESSAGE))
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

/// Return the tokens that the given `draft_attr` `attribute` carries.
fn draft_attr_tokens(attribute: &Attribute) -> Result<TokenStream> {
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

        let mut carries_validation = false;
        // A serde attribute that the walk cannot read carries no key at all, and the draft then mirrors nothing
        let _walk: Result<()> = attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident(TRY_FROM_KEY) {
                carries_validation = true;
            }
            // The walk owns the value of the key, which ends at the next key of the attribute
            while !meta.input.is_empty() && !meta.input.peek(Token![,]) {
                let _value: TokenTree = meta.input.parse()?;
            }

            Ok(())
        });

        carries_validation
    })
}

/// Return the generic parameters of `generics` that reach an error payload of `fields`, of `variants` or of `final_validations`.
/// The returned parameters keep their declaration order.
/// An empty result means that the generated error enum stays free of every parameter.
fn used_error_parameters<'generics>(
    generics: &'generics Generics,
    fields: &[FieldIntermediateRepresentation],
    variants: &[VariantIntermediateRepresentation],
    final_validations: &[FinalValidation],
) -> Vec<&'generics GenericParam> {
    generics
        .params
        .iter()
        .filter(|&parameter| {
            error_payload_types(fields, variants, final_validations)
                .any(|payload_type| names_parameter(payload_type, parameter))
        })
        .collect()
}

/// Return the tokens of every type that reaches the generated error enum of a validated type.
/// A nested field contributes its declared type, whose own error the enum wraps.
/// A nested variant payload contributes its declared type the same way.
/// A final validation contributes the error type that it returns.
fn error_payload_types(
    fields: &[FieldIntermediateRepresentation],
    variants: &[VariantIntermediateRepresentation],
    final_validations: &[FinalValidation],
) -> impl Iterator<Item = TokenStream> {
    let nested_types = fields
        .iter()
        .filter(|field| matches!(field.rule, FieldRule::Nested { .. }))
        .map(|field| field.ty.to_token_stream());
    let nested_payload_types = variants.iter().filter_map(|variant| {
        variant
            .nested_payload()
            .map(|(payload_type, _)| payload_type.to_token_stream())
    });
    let final_validation_error_types = final_validations
        .iter()
        .map(|final_validation| final_validation.error_ty.to_token_stream());

    nested_types
        .chain(nested_payload_types)
        .chain(final_validation_error_types)
}

/// Token that the parameter walk read just before the token it reads now.
/// The walk needs it to tell a lifetime from a type and to skip the segment of a foreign path.
#[derive(Clone, Copy)]
enum PrecedingToken {
    /// A token that changes the meaning of the next token in no way.
    Other,
    /// A joint colon, which becomes a path separator once a second colon follows it.
    Colon,
    /// A path separator, which makes the next identifier a segment of a foreign path.
    PathSeparator,
    /// The quote of a lifetime, whose name is the next identifier.
    Quote,
}

/// Whether the given `tokens` name the generic `parameter`.
/// A lifetime is a quote followed by its name, so the walk reads the two tokens together.
/// An identifier that follows a path separator names a segment of a foreign path, so it counts for no parameter.
fn names_parameter(tokens: TokenStream, parameter: &GenericParam) -> bool {
    let name = parameter_name(parameter);
    let wants_a_lifetime = matches!(parameter, GenericParam::Lifetime(_));
    let mut preceding = PrecedingToken::Other;
    for token in tokens {
        match token {
            TokenTree::Group(group) => {
                if names_parameter(group.stream(), parameter) {
                    return true;
                }
                preceding = PrecedingToken::Other;
            }
            TokenTree::Ident(ident) => {
                let reads_a_lifetime = matches!(preceding, PrecedingToken::Quote);
                let is_qualified = matches!(preceding, PrecedingToken::PathSeparator);
                if reads_a_lifetime == wants_a_lifetime && !is_qualified && ident == *name {
                    return true;
                }
                preceding = PrecedingToken::Other;
            }
            TokenTree::Punct(punct) => {
                preceding = match (punct.as_char(), preceding) {
                    ('\'', _) => PrecedingToken::Quote,
                    (':', PrecedingToken::Colon) => PrecedingToken::PathSeparator,
                    (':', _) if matches!(punct.spacing(), Spacing::Joint) => PrecedingToken::Colon,
                    _ => PrecedingToken::Other,
                };
            }
            TokenTree::Literal(_) => preceding = PrecedingToken::Other,
        }
    }

    false
}

/// Return the name of the given generic `parameter`.
/// The name of a lifetime parameter comes without its quote.
fn parameter_name(parameter: &GenericParam) -> &Ident {
    match parameter {
        GenericParam::Lifetime(lifetime_parameter) => &lifetime_parameter.lifetime.ident,
        GenericParam::Type(type_parameter) => &type_parameter.ident,
        GenericParam::Const(const_parameter) => &const_parameter.ident,
    }
}

/// Check that the generated error enum of a validated type carries either every parameter of `generics` or none.
/// The `used_parameters` are the parameters that the scan found in an error payload.
/// A parameter that reaches no payload while another parameter reaches one would become an unused parameter of the generated enum,
/// which the caller of the derive cannot fix, so the derive rejects it here.
/// Each unused parameter carries its own error, so a single compilation reports all of them.
fn check_error_parameters(generics: &Generics, used_parameters: &[&GenericParam]) -> Result<()> {
    if used_parameters.is_empty() {
        return Ok(());
    }
    // The scan borrows the parameters of the same generics, so the address of a parameter identifies it
    let errors: Vec<Error> = generics
        .params
        .iter()
        .filter(|parameter| {
            !used_parameters
                .iter()
                .any(|&used_parameter| ptr::eq(used_parameter, *parameter))
        })
        .map(|parameter| Error::new_spanned(parameter, ERROR_PAYLOAD_SUBSET_MESSAGE))
        .collect();

    accumulated(errors)
}

/// Check that the field enum variants generated for `fields` are all distinct.
fn check_field_enum_variants(fields: &[FieldIntermediateRepresentation]) -> Result<()> {
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

/// Check that the wrapper variants generated for `fields`, for `variants` and for `final_validations` are all distinct.
fn check_wrapper_variants(
    fields: &[FieldIntermediateRepresentation],
    variants: &[VariantIntermediateRepresentation],
    final_validations: &[FinalValidation],
) -> Result<()> {
    let mut wrapper_variants: Vec<Ident> = Vec::new();
    for field in fields {
        if let FieldRule::Nested { wrapper_variant } = &field.rule {
            wrapper_variants.push(wrapper_variant.clone());
        }
    }
    for variant in variants {
        if let Some((_, wrapper_variant)) = variant.nested_payload() {
            wrapper_variants.push(wrapper_variant.clone());
        }
    }
    for final_validation in final_validations {
        wrapper_variants.push(final_validation.wrapper_variant.clone());
    }

    let Some((first, second)) = naming::first_collision(&wrapper_variants) else {
        return Ok(());
    };
    let mut error = Error::new(
        second.span(),
        format!("the generated error variant `{second}` would be generated twice"),
    );
    // A field, a variant and a final validation reach this check in their own order,
    // so the note claims no order between the two spans
    error.combine(Error::new(
        first.span(),
        format!("the same `{first}` variant also comes from here"),
    ));

    Err(error)
}

#[cfg(test)]
mod tests {
    use proc_macro2::{Ident, Span};
    use syn::{DeriveInput, Generics, Member, parse_str};

    use crate::intermediate_representation::{
        FieldIntermediateRepresentation, FieldRule, FinalValidation, Shape,
        TypeIntermediateRepresentation, VariantKind, VariantRule,
    };

    use super::{
        ERROR_PAYLOAD_SUBSET_MESSAGE, PAYLOAD_MARKER_MESSAGE, UNION_MESSAGE,
        VARIANT_MARKER_MESSAGE, VARIANT_SHAPE_MESSAGE, parameter_name, parse,
        used_error_parameters,
    };

    /// Build an identifier from `name`, with the call site span.
    fn ident(name: &str) -> Ident {
        Ident::new(name, Span::call_site())
    }

    /// Build a nested field whose declared type is `declared_type`.
    fn nested_field(declared_type: &str) -> FieldIntermediateRepresentation {
        FieldIntermediateRepresentation {
            member: Member::Named(ident("inner")),
            logical_name: "inner".to_owned(),
            variant: None,
            ty: parse_str(declared_type).expect("the tested field type must parse"),
            docs: Vec::new(),
            passthrough: Vec::new(),
            rule: FieldRule::Nested {
                wrapper_variant: ident("InnerValidationError"),
            },
        }
    }

    /// Build a final validation whose error type is `error_type`.
    fn final_validation(error_type: &str) -> FinalValidation {
        FinalValidation {
            fn_ident: ident("validate_all"),
            error_ty: parse_str(error_type).expect("the tested error type must parse"),
            wrapper_variant: ident("AllValidationError"),
        }
    }

    /// Return the names of the parameters of `generics` that reach an error payload of `fields` or of `final_validations`.
    /// The names come joined by a comma, and an empty text means that the scan found no parameter.
    fn used_names(
        generics: &str,
        fields: &[FieldIntermediateRepresentation],
        final_validations: &[FinalValidation],
    ) -> String {
        let parsed_generics: Generics =
            parse_str(generics).expect("the tested generics must parse");

        used_error_parameters(&parsed_generics, fields, &[], final_validations)
            .into_iter()
            .map(|parameter| parameter_name(parameter).to_string())
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// Return the representation that the parsing of `source` produces.
    /// The source must parse as a derive input and the derive must accept it.
    fn accepted(source: &str) -> TypeIntermediateRepresentation {
        let derive_input: DeriveInput =
            parse_str(source).expect("the tested derive input must parse");

        parse(&derive_input).expect("the tested derive input must be accepted")
    }

    /// Return the messages of the rejections that the parsing of `source` produces.
    /// The source must parse as a derive input and the derive must reject it.
    fn rejection_messages(source: &str) -> Vec<String> {
        let derive_input: DeriveInput =
            parse_str(source).expect("the tested derive input must parse");
        let Err(error) = parse(&derive_input) else {
            panic!("the tested derive input must be rejected");
        };

        error
            .into_iter()
            .map(|single_error| single_error.to_string())
            .collect()
    }

    /// Return the description of every variant of `intermediate_representation`, in declaration order.
    /// A variant reads as its name, then its content, then the wrapper variant of a nested payload.
    /// The descriptions come joined by a comma.
    fn variant_summary(intermediate_representation: &TypeIntermediateRepresentation) -> String {
        intermediate_representation
            .variants
            .iter()
            .map(|variant| match &variant.kind {
                VariantKind::Unit => format!("{} unit", variant.ident),
                VariantKind::Payload {
                    rule: VariantRule::Skip,
                    ..
                } => format!("{} skip", variant.ident),
                VariantKind::Payload {
                    rule: VariantRule::Nested { wrapper_variant },
                    ..
                } => format!("{} nested {wrapper_variant}", variant.ident),
            })
            .collect::<Vec<String>>()
            .join(", ")
    }

    #[test]
    fn no_parameter_reaches_an_error_payload() {
        let fields = [nested_field("Inner")];
        let final_validations = [final_validation("MassError")];

        assert!(
            used_names("<Number>", &fields, &final_validations).is_empty(),
            "A payload type that names no parameter must leave the result empty"
        );
    }

    #[test]
    fn every_parameter_reaches_an_error_payload() {
        let fields = [nested_field("Inner<'data, Number, LENGTH>")];

        assert_eq!(
            used_names("<'data, Number, const LENGTH: usize>", &fields, &[]),
            "data, Number, LENGTH",
            "A payload type that names every parameter must give the whole parameter list"
        );
    }

    #[test]
    fn a_proper_subset_of_the_parameters_reaches_an_error_payload() {
        let fields = [nested_field("Inner<Number>")];

        assert_eq!(
            used_names("<Number, Other>", &fields, &[]),
            "Number",
            "Only the parameter that a payload type names must reach the result"
        );
    }

    #[test]
    fn a_qualified_identifier_names_no_parameter() {
        let fields = [nested_field("other::T")];

        assert!(
            used_names("<T>", &fields, &[]).is_empty(),
            "An identifier that follows a path separator must count for no parameter"
        );
    }

    #[test]
    fn a_lifetime_parameter_reaches_an_error_payload() {
        let fields = [nested_field("Inner<'data>")];

        assert_eq!(
            used_names("<'data>", &fields, &[]),
            "data",
            "A lifetime that a payload type names must reach the result"
        );
    }

    #[test]
    fn a_const_parameter_reaches_an_error_payload() {
        let final_validations = [final_validation("MassError<LENGTH>")];

        assert_eq!(
            used_names("<const LENGTH: usize>", &[], &final_validations),
            "LENGTH",
            "A const parameter that an error type names must reach the result"
        );
    }

    #[test]
    fn a_parameter_inside_a_projection_reaches_an_error_payload() {
        let fields = [nested_field("<Wrapper<T> as Trait>::Assoc")];

        assert_eq!(
            used_names("<T>", &fields, &[]),
            "T",
            "A parameter inside the qualified self type of a projection must reach the result"
        );
    }

    #[test]
    fn a_generic_struct_without_a_bound_is_accepted() {
        let intermediate_representation =
            accepted("struct Wrapper<Number> { #[validate(skip)] value: Number }");

        assert_eq!(
            intermediate_representation.generics.params.len(),
            1,
            "The representation must carry the parameter of the validated type"
        );
        assert!(
            !intermediate_representation.error_enum_is_generic,
            "A parameter that reaches no error payload must leave the error enum free of it"
        );
    }

    #[test]
    fn every_parameter_that_reaches_an_error_payload_makes_the_error_enum_generic() {
        let intermediate_representation =
            accepted("struct Wrapper<Number> { #[validate(nested)] inner: Inner<Number> }");

        assert!(
            intermediate_representation.error_enum_is_generic,
            "A parameter that reaches an error payload must make the error enum generic"
        );
    }

    #[test]
    fn a_proper_subset_of_the_parameters_is_rejected_at_each_unused_parameter() {
        assert_eq!(
            rejection_messages(
                "struct Wrapper<Number, Other, Extra> { #[validate(nested)] inner: Inner<Number> }"
            ),
            vec![
                ERROR_PAYLOAD_SUBSET_MESSAGE.to_owned(),
                ERROR_PAYLOAD_SUBSET_MESSAGE.to_owned(),
            ],
            "Each parameter that reaches no error payload must carry its own rejection"
        );
    }

    #[test]
    fn an_enum_with_a_unit_a_skip_and_a_nested_variant_is_accepted() {
        let intermediate_representation = accepted(
            "enum Command { Halt, Raw(#[validate(skip)] u8), Extend(#[validate(nested)] Fraction) }",
        );

        assert!(
            matches!(intermediate_representation.shape, Shape::Enum),
            "An enum must carry the enum shape"
        );
        assert!(
            intermediate_representation.fields.is_empty(),
            "An enum must carry no field at all"
        );
        assert_eq!(
            variant_summary(&intermediate_representation),
            "Halt unit, Raw skip, Extend nested ExtendValidationError",
            "Every variant must keep its declaration order, its rule and its wrapper variant"
        );
    }

    #[test]
    fn an_enum_of_unit_variants_carries_no_wrapper_variant() {
        let intermediate_representation = accepted("enum CelestialBodyKind { Sun, Earth }");

        assert!(
            intermediate_representation
                .variants
                .iter()
                .all(|variant| variant.nested_payload().is_none()),
            "An enum of unit variants must reach no wrapper variant, which leaves the generated error enum empty"
        );
    }

    #[test]
    fn a_parameter_that_reaches_a_variant_payload_makes_the_error_enum_generic() {
        let intermediate_representation = accepted(
            "enum Sample<Number> { Missing, Measured(#[validate(nested)] Measurement<Number>) }",
        );

        assert!(
            intermediate_representation.error_enum_is_generic,
            "A parameter that reaches a variant payload must make the error enum generic"
        );
    }

    #[test]
    fn a_parameter_that_reaches_no_variant_payload_is_rejected() {
        assert_eq!(
            rejection_messages(
                "enum Sample<Number, Other> { Measured(#[validate(nested)] Measurement<Number>) }"
            ),
            vec![ERROR_PAYLOAD_SUBSET_MESSAGE.to_owned()],
            "The parameter that reaches no variant payload must carry its own rejection"
        );
    }

    #[test]
    fn a_union_is_rejected() {
        assert_eq!(
            rejection_messages("union Number { floating: f64, integer: u64 }"),
            vec![UNION_MESSAGE.to_owned()],
            "A union must be rejected, because it holds no validated shape"
        );
    }

    #[test]
    fn a_variant_with_named_fields_is_rejected() {
        assert_eq!(
            rejection_messages("enum Command { Extend { fraction: f64 } }"),
            vec![VARIANT_SHAPE_MESSAGE.to_owned()],
            "A variant with named fields must be rejected"
        );
    }

    #[test]
    fn a_variant_with_two_payloads_is_rejected() {
        assert_eq!(
            rejection_messages("enum Command { Extend(f64, f64) }"),
            vec![VARIANT_SHAPE_MESSAGE.to_owned()],
            "A tuple variant with more than one payload must be rejected"
        );
    }

    #[test]
    fn a_variant_without_a_payload_is_rejected() {
        assert_eq!(
            rejection_messages("enum Command { Extend() }"),
            vec![VARIANT_SHAPE_MESSAGE.to_owned()],
            "A tuple variant without a payload must be rejected"
        );
    }

    #[test]
    fn a_range_marker_on_a_payload_is_rejected() {
        assert_eq!(
            rejection_messages("enum Command { Extend(#[validate(range(0.0..=1.0))] f64) }"),
            vec![PAYLOAD_MARKER_MESSAGE.to_owned()],
            "A range marker on a payload must be rejected, because a variant declares no rule"
        );
    }

    #[test]
    fn a_finite_marker_on_a_payload_is_rejected() {
        assert_eq!(
            rejection_messages("enum Command { Extend(#[validate(finite)] f64) }"),
            vec![PAYLOAD_MARKER_MESSAGE.to_owned()],
            "A finite marker on a payload must be rejected, because a variant declares no rule"
        );
    }

    #[test]
    fn a_payload_without_a_marker_is_rejected() {
        assert_eq!(
            rejection_messages("enum Command { Extend(f64) }"),
            vec![PAYLOAD_MARKER_MESSAGE.to_owned()],
            "A payload without a marker must be rejected, so no payload escapes the grammar"
        );
    }

    #[test]
    fn a_marker_on_a_unit_variant_is_rejected() {
        assert_eq!(
            rejection_messages("enum Command { #[validate(skip)] Halt }"),
            vec![VARIANT_MARKER_MESSAGE.to_owned()],
            "A marker on a unit variant must be rejected, because the variant carries no payload"
        );
    }

    #[test]
    fn a_variant_wrapper_that_collides_with_a_final_validation_wrapper_is_rejected() {
        assert_eq!(
            rejection_messages(
                "#[final_validation(validate_extend, error = ExtendError)] \
                 enum Command { Extend(#[validate(nested)] Fraction) }"
            ),
            vec![
                "the generated error variant `ExtendValidationError` would be generated twice"
                    .to_owned(),
                "the same `ExtendValidationError` variant also comes from here".to_owned(),
            ],
            "A variant wrapper and a final validation wrapper that share a name must be rejected"
        );
    }
}
