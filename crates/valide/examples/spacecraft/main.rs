//! Example that builds a spacecraft from a draft, patches a field through a validated setter and serialize/deserialize it.

use crate::model::{
    CelestialBodyKind, InertiaMatrixSerializableDraft, ShadowFractionDraft, Spacecraft,
    SpacecraftDraft,
};

pub mod model;

fn main() {
    let spacecraft_draft = SpacecraftDraft {
        mass: 1000.0,
        bus_mass: 600.0,
        sail_mass: 300.0,
        area: 12.5,
        inertia_matrix: InertiaMatrixSerializableDraft {
            xx: 2.0,
            xy: 0.0,
            xz: 0.0,
            yx: 0.0,
            yy: 3.0,
            yz: 0.0,
            zx: 0.0,
            zy: 0.0,
            zz: 4.0,
        },
        sun_shadow_fraction: ShadowFractionDraft(0.5),
        primary_orbited_body: CelestialBodyKind::Earth,
    };

    let mut spacecraft =
        Spacecraft::new(spacecraft_draft).expect("The valid draft must build a spacecraft");

    // The setter validates a draft that carries the new value and commits the value on success only
    spacecraft
        .set_bus_mass(650.0)
        .expect("New bus mass should be valid");

    // Serde is fully supported using `#[serde(try_from = "SpacecraftDraft<Type>")]` so deserialization will run the validation steps as well
    let serialized_spacecraft = serde_json::to_value(&spacecraft)
        .expect("A valid spacecraft must serialize")
        .to_string();
    let deserialized_spacecraft = serde_json::from_str::<Spacecraft>(&serialized_spacecraft)
        .expect("The valid document must deserialize");

    let inequality_text = if deserialized_spacecraft != spacecraft {
        " not"
    } else {
        ""
    };
    println!("The deserialized spacecraft is{inequality_text} equal to the initial spacecraft");
}
