//! The lamps of a car: its locators and the lamp records that use them. Spec: `docs/formats/chameleon-lamps.md`.
//!
//! The graphics spec holds a table of **locator groups**: a group is a numbered list of placements (a rotation and a
//! position each, in the engine's vehicle space, x left, y up, z forward). The car's `VehicleVfxBehaviour` Genesys object
//! lists the lamp records that point at a group: **coronas** (the lens flares seen from outside) and **lights** (the
//! beams a lamp throws on the road). Each record names the group it sits at and the bit of the vehicle's light-state word
//! that switches it on. Which bit means what is documented in the spec; this module only reads.
//!
//! The lamp lens meshes (`Refraction` shader) are also split here: one mesh holds every lit strip of a car, and each
//! connected piece is one lamp ([`split_lenses`]).

use std::collections::HashMap;

use chameleon_bundle::Bundle;

use crate::appearance::{LampEnd, LampSide, LampSurface, SurfaceKind};
use crate::error::{Error, Result};
use crate::genesys::{Object, TypeNames, find_object};
use crate::spec::find_spec;
use crate::vehicle::PartMesh;

const VFX_TYPE: &str = "Genesys.Gen.VehicleVfxBehaviour";
const CORONA_TYPE: &str = "Genesys.Gen.VehicleVfxBehaviour.Corona";
const LIGHT_TYPE: &str = "Genesys.Gen.VehicleVfxBehaviour.Light";
const SPOT_TYPE: &str = "Genesys.Gen.VehicleVfxBehaviour.SpotEffect";

/// Spec header: number of groups, offset of their ids (u32 each), offset of their entries (12 bytes each).
const GROUP_COUNT_AT: usize = 0x1C;
const GROUP_IDS_AT: usize = 0x20;
const GROUP_ENTRIES_AT: usize = 0x24;
/// A group entry: locator count, offset of the locators, one more word (not decoded).
const GROUP_ENTRY_LEN: usize = 12;
/// A locator: a quaternion (x, y, z, w) and a position (x, y, z, 0).
const LOCATOR_LEN: usize = 32;
const MAX_GROUPS: usize = 256;
const MAX_LOCATORS: usize = 64;

/// A placement of a lamp or another part, in the engine's vehicle space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Locator {
    /// Quaternion x, y, z, w.
    pub rotation: [f32; 4],
    /// Metres: x left, y up, z forward.
    pub position: [f32; 3],
}

impl Locator {
    /// Where the locator's local +z axis points, in the same space as the position: the way a lamp shines.
    pub fn forward(&self) -> [f32; 3] {
        let [x, y, z, w] = self.rotation;
        [2.0 * (x * z + w * y), 2.0 * (y * z - w * x), 1.0 - 2.0 * (x * x + y * y)]
    }
}

/// A numbered list of locators.
#[derive(Debug, Clone, PartialEq)]
pub struct LocatorGroup {
    pub id: u32,
    pub locators: Vec<Locator>,
}

/// One lamp record of the car's `VehicleVfxBehaviour`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LampEntry {
    /// The locator group the lamp sits at.
    pub group: u32,
    /// Which bit (0 to 31) of the vehicle's light-state word switches the lamp on.
    pub state_bit: u16,
    /// 0 or 1: the group's locators as they are; 2: their mirror image across the centre plane (reading unconfirmed).
    pub mode: u16,
    /// The record's colour factor (1, 1, 1, 1 for most records).
    pub tint: [f32; 4],
    /// The float after the group id: the intensity of a light, a size factor of a corona.
    pub level: f32,
}

/// One spot effect record of the car's `VehicleVfxBehaviour`: a particle effect played at a locator group while a bit of
/// the vehicle's state word is set. The tail pipes are the groups of the nitrous effects (bits 8 and 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpotEffect {
    /// The locator group the effect plays at.
    pub group: u32,
    /// Which bit of the vehicle's state word starts the effect.
    pub state_bit: u16,
    /// The effect is also played at the mirror image of each locator of the group (byte +0x1D).
    pub mirrored: bool,
}

/// The locators and lamp records of a car.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lamps {
    pub groups: Vec<LocatorGroup>,
    pub coronas: Vec<LampEntry>,
    pub lights: Vec<LampEntry>,
    pub spot_effects: Vec<SpotEffect>,
}

impl Lamps {
    /// The group with id `id`.
    pub fn group(&self, id: u32) -> Option<&LocatorGroup> {
        self.groups.iter().find(|group| group.id == id)
    }
}

/// Read the locator groups and the lamp records of a vehicle bundle. A bundle with no lamp records has empty lists.
pub fn read_lamps(bundle: &Bundle) -> Result<Lamps> {
    let spec = find_spec(bundle).ok_or(Error::NoSpec)?;
    let spec = bundle.resource_by_id(spec)?;
    let groups = read_groups(&spec.blocks[0])?;
    let types = TypeNames::read(bundle)?;
    let object = match find_object(bundle, &types, VFX_TYPE) {
        Ok(object) => object,
        Err(Error::Spec(_)) => return Ok(Lamps { groups, ..Lamps::default() }),
        Err(other) => return Err(other),
    };
    let coronas = entries(&object, CORONA_TYPE, 0x50, Layout::Corona)?;
    let lights = entries(&object, LIGHT_TYPE, 0x40, Layout::Light)?;
    let spot_effects = spots(&object)?;
    Ok(Lamps { groups, coronas, lights, spot_effects })
}

fn word(block: &[u8], at: usize) -> Result<u32> {
    let bytes = block.get(at..at + 4).ok_or_else(|| Error::Spec(format!("block ends before 0x{at:X}")))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn float(block: &[u8], at: usize) -> Result<f32> {
    Ok(f32::from_bits(word(block, at)?))
}

/// The locator groups of the main block of a graphics spec.
pub fn read_groups(spec: &[u8]) -> Result<Vec<LocatorGroup>> {
    let count = word(spec, GROUP_COUNT_AT)? as usize;
    if count > MAX_GROUPS {
        return Err(Error::Spec(format!("{count} locator groups")));
    }
    let (ids, entries) = (word(spec, GROUP_IDS_AT)? as usize, word(spec, GROUP_ENTRIES_AT)? as usize);
    let mut groups = Vec::with_capacity(count);
    for n in 0..count {
        let id = word(spec, ids + 4 * n)?;
        let entry = entries + GROUP_ENTRY_LEN * n;
        let (locators, at) = (word(spec, entry)? as usize, word(spec, entry + 4)? as usize);
        if locators > MAX_LOCATORS {
            return Err(Error::Spec(format!("a locator group of {locators}")));
        }
        let mut list = Vec::with_capacity(locators);
        for j in 0..locators {
            let base = at + LOCATOR_LEN * j;
            let rotation =
                [float(spec, base)?, float(spec, base + 4)?, float(spec, base + 8)?, float(spec, base + 12)?];
            let position = [float(spec, base + 16)?, float(spec, base + 20)?, float(spec, base + 24)?];
            let finite = rotation.iter().chain(&position).all(|v| v.is_finite());
            if !finite {
                return Err(Error::Spec(format!("locator group {id:X} has a locator that is not a number")));
            }
            list.push(Locator { rotation, position });
        }
        groups.push(LocatorGroup { id, locators: list });
    }
    Ok(groups)
}

#[derive(Clone, Copy)]
enum Layout {
    Corona,
    Light,
}

/// The records of one structure type, in memory order. Both layouts keep the colour at +0x10, the group id at +0x30 and
/// the float after it; a corona has its bit at +0x40 and mode at +0x42, a light at +0x38 and +0x3A.
fn entries(object: &Object, name: &str, size: usize, layout: Layout) -> Result<Vec<LampEntry>> {
    let (bit_at, mode_at) = match layout {
        Layout::Corona => (0x40, 0x42),
        Layout::Light => (0x38, 0x3A),
    };
    let mut out = Vec::new();
    for at in object.all(name) {
        if at + size > object.main.len() {
            return Err(Error::Spec(format!("a {name} record runs past its object")));
        }
        out.push(LampEntry {
            group: object.u32(at + 0x30)?,
            state_bit: object.u16(at + bit_at)?,
            mode: object.u16(at + mode_at)?,
            tint: object.floats::<4>(at + 0x10)?,
            level: object.f32(at + 0x34)?,
        });
    }
    Ok(out)
}

/// The spot effect records: group id at +0x14, state bit at +0x18, a mirror flag at +0x1D.
fn spots(object: &Object) -> Result<Vec<SpotEffect>> {
    let mut out = Vec::new();
    for at in object.all(SPOT_TYPE) {
        if at + 0x20 > object.main.len() {
            return Err(Error::Spec("a spot effect record runs past its object".into()));
        }
        let flag = object.main[at + 0x1D];
        out.push(SpotEffect { group: object.u32(at + 0x14)?, state_bit: object.u16(at + 0x18)?, mirrored: flag == 1 });
    }
    Ok(out)
}

/// Replace each lens mesh (the `Refraction` shader) by one mesh per connected piece, each marked as a lamp surface of the
/// end and side of the car it sits on. Other meshes are returned as they are.
pub fn split_lenses(mesh: PartMesh) -> Vec<PartMesh> {
    let Some(groups) = lens_groups(&mesh) else { return vec![mesh] };
    groups
        .iter()
        .map(|ids| {
            let triangles: Vec<[u32; 3]> = ids.iter().map(|&at| mesh.triangles[at]).collect();
            piece(&mesh, &triangles)
        })
        .collect()
}

/// The pieces [`split_lenses`] makes of `mesh`, each as the positions (in `mesh.triangles`, in order) of its triangles,
/// in the order the pieces come out. `None` for a mesh that is not a lens, which stays one mesh. The skin of the body
/// (`skin.rs`) follows the same split so that its triangle classes line up with the meshes.
pub(crate) fn lens_groups(mesh: &PartMesh) -> Option<Vec<Vec<usize>>> {
    let is_lens = mesh.appearance.kind == SurfaceKind::Glass && mesh.appearance.technique.contains("Refraction");
    if !is_lens || mesh.triangles.is_empty() {
        return None;
    }
    let mut pieces: HashMap<usize, Vec<usize>> = HashMap::new();
    let root = components(mesh);
    for (at, triangle) in mesh.triangles.iter().enumerate() {
        pieces.entry(root[triangle[0] as usize]).or_default().push(at);
    }
    let mut roots: Vec<usize> = pieces.keys().copied().collect();
    roots.sort_unstable();
    Some(roots.into_iter().map(|key| pieces.remove(&key).unwrap_or_default()).collect())
}

/// For each vertex, a number shared by every vertex of its connected piece (vertices at one place are one).
fn components(mesh: &PartMesh) -> Vec<usize> {
    let key = |p: [f32; 3]| p.map(|c| (c * 1000.0).round() as i64);
    let mut places: HashMap<[i64; 3], usize> = HashMap::new();
    let place: Vec<usize> = mesh
        .vertices
        .iter()
        .map(|v| {
            let next = places.len();
            *places.entry(key(v.position)).or_insert(next)
        })
        .collect();
    let mut parent: Vec<usize> = (0..places.len()).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    for triangle in &mesh.triangles {
        let [a, b, c] = triangle.map(|v| place[v as usize]);
        for other in [b, c] {
            let (ra, ro) = (find(&mut parent, a), find(&mut parent, other));
            parent[ro] = ra;
        }
    }
    place.iter().map(|&p| find(&mut parent, p)).collect()
}

/// One piece of a lens as a mesh of its own, with compact vertices and the lamp surface its place says.
fn piece(mesh: &PartMesh, triangles: &[[u32; 3]]) -> PartMesh {
    let mut remap: HashMap<u32, u32> = HashMap::new();
    let mut vertices = Vec::new();
    let mut out = Vec::with_capacity(triangles.len());
    for triangle in triangles {
        out.push(triangle.map(|v| {
            *remap.entry(v).or_insert_with(|| {
                vertices.push(mesh.vertices[v as usize]);
                (vertices.len() - 1) as u32
            })
        }));
    }
    let n = vertices.len() as f32;
    let mean = |axis: usize| vertices.iter().map(|v| v.position[axis]).sum::<f32>() / n;
    let surface = lamp_surface(mean(0), mean(2));
    let mut appearance = mesh.appearance.clone();
    appearance.kind = SurfaceKind::Lamp(surface);
    PartMesh { vertices, triangles: out, appearance }
}

/// Half the width of the strip along the centre plane that counts as the middle of the car, metres.
const CENTRE_BAND: f32 = 0.12;

/// The end and side of a lamp piece centred at `x` (left) and `z` (forward) of the engine's vehicle space.
fn lamp_surface(x: f32, z: f32) -> LampSurface {
    let end = if z >= 0.0 { LampEnd::Front } else { LampEnd::Rear };
    let side = match x {
        x if x > CENTRE_BAND => LampSide::Left,
        x if x < -CENTRE_BAND => LampSide::Right,
        _ => LampSide::Centre,
    };
    LampSurface { end, side }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::Appearance;
    use chameleon_gfx::Vertex;

    fn vertex(position: [f32; 3]) -> Vertex {
        Vertex { position, color: [255; 4], uv: [0.0; 2] }
    }

    fn lens(triangles: &[[[f32; 3]; 3]]) -> PartMesh {
        let vertices: Vec<Vertex> = triangles.iter().flatten().map(|&p| vertex(p)).collect();
        let triangles = (0..triangles.len() as u32).map(|t| [3 * t, 3 * t + 1, 3 * t + 2]).collect();
        let appearance = Appearance {
            kind: SurfaceKind::Glass,
            technique: "VehicleNFS13_Refraction".into(),
            diffuse_texture: None,
            diffuse_colour: None,
            alpha_test: false,
        };
        PartMesh { vertices, triangles, appearance }
    }

    fn strip(x: f32, z: f32) -> [[f32; 3]; 3] {
        [[x, 0.7, z], [x + 0.1, 0.7, z], [x, 0.7, z + 0.1]]
    }

    #[test]
    fn a_locator_with_no_rotation_shines_forward_and_one_turned_half_a_circle_shines_back() {
        let ahead = Locator { rotation: [0.0, 0.0, 0.0, 1.0], position: [0.0; 3] };
        let behind = Locator { rotation: [0.0, 1.0, 0.0, 0.0], position: [0.0; 3] };
        assert_eq!(ahead.forward(), [0.0, 0.0, 1.0]);
        assert_eq!(behind.forward(), [0.0, 0.0, -1.0]);
    }

    #[test]
    fn a_lens_splits_into_its_pieces_with_a_place_each() {
        // Front left, front right and a rear centre strip, apart from each other.
        let pieces = split_lenses(lens(&[strip(0.5, 2.0), strip(-0.6, 2.0), strip(-0.05, -2.0)]));
        assert_eq!(pieces.len(), 3);
        let surfaces: Vec<SurfaceKind> = pieces.iter().map(|p| p.appearance.kind).collect();
        let lamp = |end, side| SurfaceKind::Lamp(LampSurface { end, side });
        assert!(surfaces.contains(&lamp(LampEnd::Front, LampSide::Left)));
        assert!(surfaces.contains(&lamp(LampEnd::Front, LampSide::Right)));
        assert!(surfaces.contains(&lamp(LampEnd::Rear, LampSide::Centre)));
        assert!(pieces.iter().all(|p| p.triangles.len() == 1 && p.vertices.len() == 3));
    }

    #[test]
    fn triangles_that_share_a_corner_stay_one_piece() {
        let a = [[0.5, 0.7, 2.0], [0.6, 0.7, 2.0], [0.5, 0.7, 2.1]];
        let b = [[0.6, 0.7, 2.0], [0.7, 0.7, 2.0], [0.6, 0.7, 2.1]];
        assert_eq!(split_lenses(lens(&[a, b])).len(), 1);
    }

    #[test]
    fn other_meshes_are_left_alone() {
        let mut mesh = lens(&[strip(0.5, 2.0)]);
        mesh.appearance.technique = "VehicleNFS13_Glass".into();
        assert_eq!(split_lenses(mesh.clone()), vec![mesh]);
    }

    #[test]
    fn groups_are_read_from_the_spec_tables() {
        let mut spec = vec![0u8; 0x100];
        let put = |spec: &mut Vec<u8>, at: usize, value: u32| spec[at..at + 4].copy_from_slice(&value.to_le_bytes());
        put(&mut spec, GROUP_COUNT_AT, 1);
        put(&mut spec, GROUP_IDS_AT, 0x40);
        put(&mut spec, GROUP_ENTRIES_AT, 0x50);
        put(&mut spec, 0x40, 0xABCD);
        put(&mut spec, 0x50, 1);
        put(&mut spec, 0x54, 0x80);
        put(&mut spec, 0x80 + 12, 1.0f32.to_bits());
        put(&mut spec, 0x80 + 16, 0.25f32.to_bits());
        put(&mut spec, 0x80 + 20, 0.5f32.to_bits());
        put(&mut spec, 0x80 + 24, 2.0f32.to_bits());
        let groups = read_groups(&spec).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, 0xABCD);
        assert_eq!(groups[0].locators[0].position, [0.25, 0.5, 2.0]);
    }

    #[test]
    fn a_table_that_runs_off_the_block_is_refused() {
        let mut spec = vec![0u8; 0x40];
        spec[GROUP_COUNT_AT] = 20;
        spec[GROUP_IDS_AT] = 0x30;
        assert!(read_groups(&spec).is_err());
    }

    /// Every car of the install has its lamp records pointing at groups it has, and nearly every car has a head lamp
    /// record (bit 0 or 6) at the front and a brake record (bit 1) at the rear (`CHAMELEON_GAME_DIR`).
    #[test]
    #[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
    fn nearly_every_high_detail_car_has_head_and_brake_lamps() {
        let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
        let (mut cars, mut complete, mut coronas, mut lights) = (0, 0, 0, 0);
        for entry in std::fs::read_dir(std::path::Path::new(&dir).join("VEHICLES")).unwrap().flatten() {
            let file = entry.file_name().to_string_lossy().to_uppercase();
            if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
                continue;
            }
            let data = std::fs::read(entry.path()).unwrap();
            let lamps = read_lamps(&Bundle::parse(&data).unwrap()).unwrap_or_else(|error| panic!("{file}: {error}"));
            let z =
                |entry: &LampEntry| lamps.group(entry.group).and_then(|g| g.locators.first()).map(|l| l.position[2]);
            let head = lamps.coronas.iter().any(|c| matches!(c.state_bit, 0 | 6) && z(c).is_some_and(|z| z > 0.0));
            let brake = lamps.coronas.iter().any(|c| c.state_bit == 1 && z(c).is_some_and(|z| z < 0.0));
            assert!(lamps.coronas.iter().chain(&lamps.lights).all(|e| lamps.group(e.group).is_some()), "{file}");
            cars += 1;
            complete += usize::from(head && brake);
            coronas += lamps.coronas.len();
            lights += lamps.lights.len();
        }
        println!("{cars} cars, {complete} with head and brake lamps, {coronas} coronas, {lights} lights");
        assert!(cars > 50 && complete + 3 >= cars, "{complete} of {cars}");
    }
}
