//! `gear.json`: the list of items the game shows, with the paths of their files under `RIDER/GEAR/`
//! (`docs/formats/ride6-rider-gear.md`, "The runtime files").

use std::collections::HashSet;

use serde_json::{Value, json};

use super::catalogue::Catalogue;

/// The folders of the runtime files, inside `GEAR/`.
pub const MESHES: &str = "meshes";
pub const TEXTURES: &str = "tex";

/// The path of a mesh's runtime file.
pub fn mesh_path(key: &str) -> String {
    format!("{MESHES}/{key}.bbsk")
}

/// The path of a picture's runtime file.
pub fn texture_path(key: &str) -> String {
    format!("{TEXTURES}/{key}.r6tx")
}

/// What was written: the keys of the meshes and pictures whose files exist.
pub struct Written {
    pub meshes: HashSet<String>,
    pub textures: HashSet<String>,
}

/// A body's name: `Male 1`, `Female 2` (the shape of the suit).
fn body_name(gender: &str, shape: u32) -> String {
    let mut chars = gender.chars();
    let head = chars.next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default();
    format!("{head}{} {shape}", chars.as_str())
}

/// The index of the items whose files were all written. An item with a missing piece is left out, and named in the second list.
pub fn build(catalogue: &Catalogue, written: &Written) -> (Value, Vec<String>) {
    let mesh = |key: &str| written.meshes.contains(key).then(|| mesh_path(key));
    let texture = |key: &str| written.textures.contains(key).then(|| texture_path(key));
    let mut left_out = Vec::new();

    let mut bodies = Vec::new();
    for body in &catalogue.bodies {
        let Some(path) = mesh(&body.mesh) else {
            left_out.push(format!("body {}: its mesh is missing", body.id));
            continue;
        };
        bodies.push(
            json!({ "id": body.id, "name": body_name(&body.gender, body.shape), "gender": body.gender, "mesh": path }),
        );
    }

    let mut helmets = Vec::new();
    for helmet in &catalogue.helmets {
        let (Some(mesh_file), Some(color)) = (mesh(&helmet.mesh), texture(&helmet.color)) else {
            left_out.push(format!("helmet {}: its mesh or picture is missing", helmet.id));
            continue;
        };
        let mut entry =
            json!({ "id": helmet.id, "name": helmet.name, "brand": helmet.brand, "mesh": mesh_file, "color": color });
        // A visor that cannot be written leaves the helmet without one.
        let visor =
            helmet.visor.as_ref().and_then(|v| Some(json!({ "mesh": mesh(&v.mesh)?, "color": texture(&v.color)? })));
        if let Some(visor) = visor {
            entry["visor"] = visor;
        }
        helmets.push(entry);
    }

    let mut suits = Vec::new();
    for suit in &catalogue.suits {
        let Some(color) = texture(&suit.color) else {
            left_out.push(format!("suit {}: its picture is missing", suit.id));
            continue;
        };
        suits.push(json!({ "id": suit.id, "name": suit.name, "brand": suit.brand, "color": color }));
    }

    let gloves: Vec<Value> = catalogue
        .gloves
        .iter()
        .map(|g| json!({ "id": g.id, "name": g.name, "brand": g.brand, "rgb": g.rgb }))
        .collect();

    let mut boots = Vec::new();
    for pair in &catalogue.boots {
        let (Some(mesh_file), Some(color)) = (mesh(&pair.mesh), texture(&pair.color)) else {
            left_out.push(format!("boots {}: its mesh or picture is missing", pair.id));
            continue;
        };
        boots.push(json!({ "id": pair.id, "name": pair.name, "brand": pair.brand, "mesh": mesh_file, "color": color }));
    }

    let index =
        json!({ "version": 1, "bodies": bodies, "helmets": helmets, "suits": suits, "gloves": gloves, "boots": boots });
    (index, left_out)
}
