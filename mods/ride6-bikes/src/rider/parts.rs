//! The pieces of the stock rider and the pictures on them. The choice of pieces is the game's default rider
//! (`default_rider/customization_data/Default_*`): suit 001, the racing helmet with its closed visor, boots 001, and the
//! gloves picture on the hands.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{Value, json};

/// `(mesh, material slot, picture, translucent, drawn)`. The mesh is the name the mesh carries inside its export (the pack's
/// mesh name, not the file's). The picture is the base colour of the material instance named in the customization asset:
/// `mat_suit001` (`T_suit001_BC`), `mat_boots001` (`boots001_d`), `mat_helmet_default` (`helmetDEfault_d`),
/// `mat_visor_default` (`visorDefault_d`). The slot name is the one the mesh itself carries. The arms mesh is not drawn:
/// the suit mesh already has arms and hands (its weights reach the fingers), and the arms mesh would double the sleeves
/// (`docs/specs/ride6-rider-render-integration.md`, section 3).
pub const PARTS: [(&str, &str, &str, bool, bool); 5] = [
    ("suit001_male", "BASE_suit", "T_suit001_BC.png", false, true),
    ("SuitRacing_Male_arms", "BASE_suit", "T_suit001_BC.png", false, false),
    ("boots001", "mat_boots006", "boots001_d.png", false, true),
    ("helmetRacing_default", "mat_helmet_default", "helmetDEfault_d.png", false, true),
    ("visorClosedRacing_default", "visor_mat", "visorDefault_d.png", true, true),
];

/// The picture of the gloves, composited onto the hands by the game's material (`MF_CompositingGloves`); not applied.
pub const GLOVES_PICTURE: &str = "T_gloves002_BC.png";

/// The parts table for the meshes `present` in the pack (by name); a part whose mesh is not among them is left out.
pub fn parts_json(present: &[String]) -> Value {
    let parts: Vec<Value> = PARTS
        .iter()
        .filter(|(mesh, ..)| present.iter().any(|name| name == mesh))
        .map(|(mesh, slot, picture, translucent, drawn)| {
            json!({ "mesh": mesh, "slot": slot, "picture": picture, "translucent": translucent, "drawn": drawn })
        })
        .collect();
    json!({ "parts": parts, "gloves_picture": GLOVES_PICTURE })
}

/// Copies every `.png` of `from` into `to`.
pub fn copy_textures(from: &Path, to: &Path) -> Result<()> {
    let entries = fs::read_dir(from).with_context(|| format!("reading {}", from.display()))?;
    for entry in entries {
        let path = entry?.path();
        if path.extension().is_some_and(|x| x == "png") {
            let name = path.file_name().context("a picture without a name")?;
            fs::copy(&path, to.join(name)).with_context(|| format!("copying {}", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_meshes_in_the_pack_are_listed_and_the_arms_are_not_drawn() {
        let present: Vec<String> = ["suit001_male", "SuitRacing_Male_arms", "visorClosedRacing_default"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let json = parts_json(&present);
        let parts = json["parts"].as_array().unwrap();
        assert_eq!(parts.len(), 3);
        let arms = parts.iter().find(|p| p["mesh"] == "SuitRacing_Male_arms").unwrap();
        assert_eq!(arms["drawn"], false);
        let visor = parts.iter().find(|p| p["mesh"] == "visorClosedRacing_default").unwrap();
        assert_eq!(visor["translucent"], true);
        assert_eq!(json["parts"][0]["drawn"], true);
        assert!(parts_json(&[])["parts"].as_array().unwrap().is_empty());
    }
}
