**A crate for types that can only be built through a validation, with validated setters to patch them.**

A validated type has no public constructor and no public fields.\
To build one you fill a draft, which is a plain mirror struct with public fields, then you hand the draft to the type.
The type validates the draft and either returns the value or returns the first error found.\
Once you hold the value, it passed every rule the type declares.

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `derive` | Yes | Re-export the two derive macros (`Validate` and `Patch`) from `valide` to generate the crate machinery. This allow to declare type rules as attributes instead of carrying hand written validation code. |

Disable the feature to take the traits alone, which drops the compile time cost of the macro dependencies.\
You then implement the traits by hand.

## Example

```rust
use valide::{Patch, Validate};

/// Error of the mass check of a spacecraft.
#[derive(Clone, PartialEq, Debug, thiserror::Error)]
pub enum MassSumError {
    /// The bus weighs more than the whole spacecraft.
    #[error("The total mass must cover the mass of the bus")]
    MassSmallerThanSum,
}

/// Reference properties of a spacecraft.
#[derive(Clone, Validate, Patch)]
#[final_validation(validate_mass_sum, error = MassSumError)]
pub struct Spacecraft {
    /// Total mass in kilograms (kg).
    #[validate(range(0.0..f64::INFINITY))]
    mass: f64,
    /// Mass of the bus in kilograms (kg).
    #[validate(range(0.0..=30_000.0))]
    bus_mass: f64,
    /// Name of the mission.
    #[validate(skip)]
    mission: String,
}

impl Spacecraft {
    /// Check that the total mass of the given `draft` covers its bus mass.
    pub fn validate_mass_sum(draft: &SpacecraftDraft) -> Result<(), MassSumError> {
        if draft.mass < draft.bus_mass {
            return Err(MassSumError::MassSmallerThanSum);
        }

        Ok(())
    }
}

fn main() {
    // Build a validated spacecraft
    let mut spacecraft = Spacecraft::new(SpacecraftDraft {
        mass: 1000.0,
        bus_mass: 600.0,
        mission: "Deep Space".to_owned(),
    })
    .expect("The given SpacecraftDraft should be valid");
}
```

## Generated items

`Validate` generates:

- The draft, the only way in.
  It carries no invariant, so you can build it field by field, read it from a file or receive it over a network.
  A `nested` field appears as the draft of its own type, named through the projection `<FieldType as Validate>::Draft`.
- One validator per validated field, callable alone to test one rule on its own.
- The aggregated function `validate`, which runs the field validators in declaration order, stops at the first error, then runs every final validation.
  A final validation holds a rule that spans several fields, so it always reads a draft whose fields are individually valid.
- The error enum.
  The shared `OutOfRange` and `NotFinite` variants carry a generated field enum that names the failing field.
  One wrapper variant exists per final validation and per fallible nested field.
- The `TryFrom` of the draft and the `new` constructor, the two validated entry points.
  When using the [`serde`](https://docs.rs/serde/latest/serde/) crate, write `#[serde(try_from = "TypeDraft")]` on the type so the whole validation also guards deserialization.
- The getters.
  A primitive field returns its value, every other field returns a reference.
- The `Validate` implementation.

`Patch` generates:
- The conversion back to a draft.
- The `Patch` implementation.
- One setter per field.
  A setter validates a draft that carries the new value and commits the value only on success.
  A rejected patch leaves the spacecraft untouched.

## The grammar

### Markers of a field

Every field must carry exactly one marker. A field without a marker is a compilation error, so
a new field never escapes the validation by accident.

- `#[validate(range(0.0..=1.0))]` accepts one range expression. The value must be inside it.
- `#[validate(range(Bound::Excluded(0.0), Bound::Excluded(f64::INFINITY)))]` accepts two bounds.
  Use this form for a range that the range syntax cannot spell, such as an excluded lower bound.
  The generated check resolves `Bound` in the module of the validated type, which must import it.
- `#[validate(finite)]` accepts a floating point value that is neither infinite nor a not a
  number value. The marker is meant for `f32` and `f64`. The generated check calls the
  `is_finite` method of the field type, which a trait bound can provide for a generic field.
- `#[validate(nested)]` delegates the validation to the type of the field, which must implement
  `Validate`. The draft of the type holds the draft of the field.
- `#[validate(skip)]` excludes the field from every field validation, so the field gets no
  validator. The setter of the field stays infallible while the type declares no final
  validation. As soon as the type declares one final validation, the setter becomes fallible.
  It then builds the draft, runs every final validation and commits the new value only on
  success, because a final validation can read a skip field.

### Attributes of a type

- `#[final_validation(function, error = ErrorType)]` runs `function` once every field validator
  passed. The function takes a reference to the draft and returns `Result<(), ErrorType>`.
  Repeat the attribute to run several functions in order.
  The function must be an inherent associated function of the validated type, because the
  generated call names it `Type::function(draft)`. A free function does not work.
- `#[draft_attr(...)]` carries its payload to the generated draft as an attribute. Use it to
  give the draft the derives that the validated type cannot infer.

### Attributes of a field

- Every `#[serde(...)]` attribute of a field also reaches the matching draft field, verbatim.
  A renamed key therefore stays the same on both sides of a deserialization validation.
- `#[draft_attr(...)]` also works on a field. The matching draft field carries its payload, the
  same way the type level attribute works on the draft itself.
- The documentation of a field reaches the draft field. A field without documentation gets a
  generated one, because every field of the draft is public.

### The skip contract

A skip field takes part in no field validation. Its setter stays infallible while the type declares
no final validation. As soon as the type declares one final validation, the setter becomes fallible
and runs every final validation, because a final validation can read a skip field.

### Newtypes

A tuple struct with one field is supported. The single field is called `value`, so the getter is
`value()`, the setter is `set_value()`, the validator is `validate_value()` and the field enum
variant is `Value`.

### Enums

An enum is supported. A variant is a unit variant, or a tuple variant with exactly one payload that carries the marker, such as `Custom(#[validate(nested)] CelestialBody)`.
A payload accepts `nested` and `skip` only.
The enum declares no rule of its own, so every rule lives in the payload type and a public variant constructor bypasses nothing.

The error enum holds one wrapper variant per nested variant, named after the variant.
No field enum exists.
An enum gets no getter, because a caller matches on the public variants.
`Patch` generates no setter, because a patch of an enum replaces the whole variant, which `new` already validates.
The draft enum takes the serde representation of serde itself, the external tagging.
Forward another representation with `#[draft_attr(serde(...))]`.

## Limitations

- A validated type must be a struct or an enum, and a union is rejected with a clear error. A variant must be a unit variant or a tuple variant with exactly one `nested` or `skip` payload.
- A tuple struct must have exactly one field.
- The by value getter uses the written token of the field type, so an alias to a primitive gets a getter that returns a reference.
- `Validate::from_draft_unchecked` skips the validation. It exists for the generated code.
  A direct call on an invalid draft builds an invalid value.
- Generics are supported with some caveats
  - A generic type gets no inferred bounds.
    Declare every bound yourself, the macro copies the generics and the where clause verbatim onto every generated item, and a missing bound fails with the ordinary compiler error.
  - A floating point literal inside a range cannot bind a generic parameter.
    Write the bounds in the parameter, such as `range(Number::ZERO..=Number::ONE)`.
  - Only a nested field type and a final validation error type carry a parameter into the error enum.
    Once one parameter reaches it, every parameter must.
    The derive rejects a proper subset with an error at each unused parameter. Remove that parameter, nest it in a validated field, or name it in a final validation error.
- A parameter inside the error enum needs `'static`, `Debug` and `Display` bounds, and `Patch` needs `Clone`.
