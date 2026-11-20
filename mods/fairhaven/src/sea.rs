//! The sea beyond the coast: one flat square of water at the sea's height under the whole world, made by the mod (it is not data of
//! the 2012 game: the city lists a sea mesh of its own in the units by the coast, with a normal map and a reflection that this
//! game's shaders cannot draw, so a unit far from the coast has no water and the horizon past the harbour was black). The file is a
//! few kilobytes and is written to the mod's folder with the manifests; the coast's own sea mesh is still drawn in each unit.

use std::fmt::Write as _;

use anyhow::Result;
use blackbox_hash::bstring_hash;
use blackbox_scenery::build::{Placement, section as scenery_section};
use blackbox_tpk::{PixelFormat, TextureSource};

use crate::manifest::FILES_FOLDER;
use crate::mesh::Part;
use crate::pvs::Zone;
use crate::section;
use crate::solid::{self, Group};

/// The manifest key of the sea (its file name without `.toml`) and its file under the mod's `files/`.
pub const KEY: &str = "sea";
pub fn file() -> String {
    format!("{FILES_FOLDER}/SEA.BUN")
}

/// The height of the sea's square, metres (the engine's y, the render space's z). The coast's sea mesh is at 10.0
/// (`docs/formats/chameleon-world.md`); the square lies a little under it so that mesh draws over it where both are.
pub const LEVEL: f32 = 9.6;
/// Half the side of the square, metres: about 60 km, past the horizon of any view.
const HALF_SIDE: f32 = 60_000.0;
/// The name and colour (RGB565, little endian) of the picture the water is drawn with: the same dark green-blue the units' sea has.
pub const WATER_NAME: &str = "FLATWATER";
pub const WATER_BLOCK: [u8; 8] = [0xCD, 0x1A, 0xCD, 0x1A, 0, 0, 0, 0];

/// The circle that holds every zone of `zones` (render space): centre x, y and radius, with `margin` metres more.
pub fn circle_of(zones: &[Zone], margin: f32) -> [f32; 3] {
    let centres: Vec<[f32; 2]> = zones.iter().map(Zone::centre_render).collect();
    if centres.is_empty() {
        return [0.0, 0.0, margin];
    }
    let mean = [0, 1].map(|axis| centres.iter().map(|c| c[axis]).sum::<f32>() / centres.len() as f32);
    let reach = zones
        .iter()
        .zip(&centres)
        .map(|(zone, c)| (c[0] - mean[0]).hypot(c[1] - mean[1]) + zone.radius())
        .fold(0.0, f32::max);
    [mean[0], mean[1], reach + margin]
}

/// The text of the sea's manifest: a pack its file places, always loaded (its anchor holds the whole world).
pub fn manifest(section: u32, [x, y, radius]: [f32; 3]) -> String {
    let mut text = String::new();
    let _ = writeln!(text, "[pack]");
    let _ = writeln!(text, "file = \"{}\"", file());
    let _ = writeln!(text, "section = {section}");
    let _ = writeln!(text, "anchor = [{x:.1}, {y:.1}, {LEVEL:.1}, {radius:.1}]");
    let _ = writeln!(text, "placed = \"file\"");
    text
}

/// The square's solid. Corners in engine space (x left, y up, z forward), in the order that makes it face up in the game's render space.
fn square() -> solid::BuiltSolid {
    let h = HALF_SIDE;
    let part = Part {
        positions: vec![[-h, LEVEL, -h], [-h, LEVEL, h], [h, LEVEL, h], [h, LEVEL, -h]],
        uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        indices: vec![0, 1, 2, 0, 2, 3],
    };
    solid::build("SEA", vec![Group { texture: WATER_NAME.to_owned(), part }]).remove(0)
}

/// The bytes of the sea's section `number`: one square solid, its picture and one placement, with no collision (a car that leaves
/// the land drives on the seabed, as it does at the coast of the units).
pub fn build(number: u32) -> Result<Vec<u8>> {
    let built = square();
    let hash = bstring_hash(&built.source.name);
    let placement = Placement {
        name: built.source.name.clone(),
        // Its own low-detail model too: an empty slot draws nothing where the game's density rule picks it.
        solids: [hash, hash, hash, 0],
        local_min: built.source.bounds_min,
        local_max: built.source.bounds_max,
        radius: built.radius,
        position: built.centre,
        rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        exclude_flags: 0x60,
    };
    let scenery =
        scenery_section(number, &[placement]).map_err(|error| anyhow::anyhow!("the sea's placement: {error:?}"))?;
    let water = TextureSource::new(WATER_NAME, 4, 4, PixelFormat::Dxt1, 1, WATER_BLOCK.to_vec());
    Ok(section::build("SEA", number, &[built.source], &[water], &[], &scenery)?.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_modding::content::parse_scenery;
    use blackbox_scenery::layout::MOST_WANTED;

    fn zone(unit: u32, min: [f32; 2], max: [f32; 2]) -> Zone {
        Zone { unit, district: 1, min, max }
    }

    #[test]
    fn the_circle_holds_every_zone_and_the_margin() {
        let zones = [zone(1, [0.0, 0.0], [100.0, 100.0]), zone(2, [900.0, 0.0], [1000.0, 100.0])];
        let [x, _, radius] = circle_of(&zones, 500.0);
        assert!(x.is_finite());
        for zone in &zones {
            let [cx, cy] = zone.centre_render();
            let [mx, my, _] = circle_of(&zones, 0.0);
            assert!((cx - mx).hypot(cy - my) + zone.radius() <= radius, "zone {} is outside the circle", zone.unit);
        }
        assert_eq!(circle_of(&[], 7.0), [0.0, 0.0, 7.0]);
    }

    #[test]
    fn the_manifest_parses_and_the_file_has_one_flat_solid_and_its_placement() {
        let pack = parse_scenery(KEY, &manifest(2909, [1000.0, 2000.0, 9000.0])).unwrap();
        assert!(pack.placed_by_file && pack.places.is_empty());
        assert_eq!((pack.section, pack.file.as_str()), (2909, "WORLDS/FAIRHAVEN/SEA.BUN"));
        assert_eq!(pack.anchor, [1000.0, 2000.0, LEVEL, 9000.0]);
        let bytes = build(2909).unwrap();
        assert!(bytes.len() < 8192, "{} bytes", bytes.len());
        let solids = blackbox_solid::read_solids(&bytes).unwrap();
        assert_eq!(solids.len(), 1);
        assert_eq!(blackbox_tpk::read_texture_packs(&bytes).unwrap()[0].textures[0].name, WATER_NAME);
        assert!(blackbox_collision::read_collision_packs(&bytes).unwrap().is_empty());
        let scenery = blackbox_scenery::read_scenery_sections(&bytes, &MOST_WANTED).unwrap();
        assert_eq!(scenery[0].instances.len(), 1);
        // The square lies at the sea's height and faces up: every vertex is at z = LEVEL in render space, and the placement
        // carries the solid's centre.
        let centre = scenery[0].instances[0].position;
        assert!((centre[2] - LEVEL).abs() <= 0.5, "centre {centre:?}");
        let up = square().source.groups[0].vertices.iter().all(|v| (v.normal[2] - 1.0).abs() < 1e-3);
        assert!(up, "the square faces down");
    }
}
