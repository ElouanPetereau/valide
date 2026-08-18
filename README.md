# Valide

[![Build Status]][actions]

[Build Status]: https://img.shields.io/github/actions/workflow/status/ElouanPETEREAU/valide/ci.yml?branch=main
[actions]: https://github.com/ElouanPETEREAU/valide/actions?query=branch%3Amain

**A crate for types that can only be built through a validation, with validated setters to patch them.**

A validated type has no public constructor and no public fields.\
To build one you fill a draft, which is a plain mirror struct with public fields, then you hand the draft to the type.
The type validates the draft and either returns the value or returns the first error found.\
Once you hold the value, it passed every rule the type declares.

Two derive macros (`Validate` and `Patch`) generate that machinery, so a type declares its rules as attributes instead of carrying hand written validation code.

---

You may be looking for:
- [An overview of valide](./OVERVIEW.md)
- [Complete Examples](./crates/valide/examples/)

## Example

The [`crates/valide/examples`](./crates/valide/examples/) folder holds a complete, runnable example.
The `spacecraft` example builds a validated spacecraft model, patches a field through a validated setter and serialize/deserialize it to a JSON document.
The unit tests of `valide` compile the same model file, so the example cannot drift from the tested code.

```bash
cargo run -p valide --example spacecraft
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