# Valide

**A crate for types that can only be built through a validation, with validated setters to patch them.**

A validated type has no public constructor and no public fields.\
To build one you fill a draft, which is a plain mirror struct with public fields, then you hand the draft to the type.
The type validates the draft and either returns the value or returns the first error found.\
Once you hold the value, it passed every rule the type declares.

Two derive macros (`Validate` and `Patch`) generate that machinery, so a type declares its rules as attributes instead of carrying hand written validation code.

---

You may be looking for:
- [An overview of valide](./OVERVIEW.md)

## Example

<details>
<summary>
Click to show Cargo.toml.
</summary>

```toml
[dependencies]

# The core APIs, including the Validate and Patch traits.
# The "derive" feature is only required when using #[derive(Validate, Patch)] to use the derives to generate most of the boilerplate code.
validate = { version = "0.1.0", features = ["derive"] }
```

</details>

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
        mission: "Sextant".to_owned(),
    })
    .expect("The given SpacecraftDraft should be valid");
}
```

## Build and test

Build the workspace.

```bash
cargo build
```

This runs the unit tests of the macros, the domain model that exercises both derives, and the compilation suite.

```bash
cargo test
```

Run the lints and the formatting.
The workspace shares one lint set, and the build must stay free of warnings.

```bash
cargo clippy --workspace --all-targets
cargo fmt --all --check
```

### Compilation tests

`valide_derive` carries a trybuild suite:
- A fail fixture is a file that must not compile, next to a `.stderr` snapshot of the exact diagnostic it must produce.
- A pass fixture is a file that must compile and run, which proves the generated code resolves from a crate that is not `valide`.

The compilation suite pins compiler diagnostics inside its `.stderr` snapshots.
A toolchain update that changes the diagnostic or a voluntary change in the expected diagnostic rendering breaks the suite.
The snapshots need to be regenerated with `TRYBUILD=overwrite`:
```bash
TRYBUILD=overwrite cargo test -p valide_derive --test ui
```