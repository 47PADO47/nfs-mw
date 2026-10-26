//! The hinged parts of a 2012 car as the car's own solids and `[[damage_parts]]` tables.
//!
//! The 2012 body is one skinned model; the parts that hang on hinges are the triangles weighted to their bones
//! (`chameleon_vehicle::body_parts`, spec `docs/formats/chameleon-hinged-parts.md`). The conversion cuts those triangles
//! out of the finest body solid into one solid per part (`<ID>_KIT00_<PART>_A`, `_B`), so the body no longer holds them,
//! and the vertices, normals and materials stay exactly as they were: an intact car draws the same pixels. Coarser body
//! levels have no skin data for most parts and keep them. The 2012 ranges are condensed into the one swing a manifest
//! entry describes ([`swing`]). An active spoiler (`chameleon_vehicle::spoiler_motion`) is the part of the bone
//! `SpoilerHinge`, with or without a damage record of its own, and carries the `[damage_parts.active]` table.
//! Spec: `docs/formats/chameleon-active-aero.md`.

use blackbox_solid::{GroupSource, SolidSource, Vertex};
use chameleon_bundle::{Bundle, Pool};
use chameleon_vehicle::{HingedPart, Skeleton, SpoilerMotion, Vehicle, body_parts, hinged_parts, spoiler_motion};

use crate::fit::{Fit, to_car_space};

/// Most thresholds a manifest entry may hold (`blackbox_modding::content::MAX_STAGES`).
const MAX_STAGES: usize = 8;

/// The hinged parts of a car and which of the finest body level's triangles belong to each.
#[derive(Debug, Clone, PartialEq)]
pub struct Hinged {
    pub parts: Vec<Part>,
    /// For each mesh of the finest body level and each of its triangles, the index into `parts`.
    pub classes: Vec<Vec<Option<usize>>>,
}

/// One part with what the 2012 data says about it, in the 2012 engine's space (x left, y up, z forward).
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// The bone's name, `BonnetHinge`.
    pub name: String,
    /// The zone code of the file, 1 to 8.
    pub zone: u16,
    /// Where the hinge bone sits.
    pub pivot: [f32; 3],
    pub limits: [[f32; 2]; 3],
    pub thresholds: Vec<f32>,
    /// The part's mass in kg: the mass of the body it becomes when it falls off.
    pub mass_kg: f32,
    /// How the part moves with the car's speed, for the active spoiler.
    pub active: Option<SpoilerMotion>,
}

/// The name of the bone of an active spoiler, and the zone code (the rear) it is given when the car has no record of it.
const SPOILER_BONE: &str = "SpoilerHinge";
const REAR_ZONE: u16 = 2;
/// What an active spoiler weighs when the car's damage object does not say, kg.
const SPOILER_MASS_KG: f32 = 30.0;

/// Read the hinged parts of the car in `bundle` and classify the triangles of the finest body level of `vehicle`. `None`
/// when the car has no usable part: no skeleton, no skin, or parts that own no triangle of the finest level.
pub fn read(bundle: &Bundle, pool: &Pool, spec: u64, vehicle: &Vehicle) -> Option<Hinged> {
    let mut parts = hinged_parts(bundle).ok()?;
    let skeleton = Skeleton::read(bundle)?;
    let motion = spoiler_motion(bundle, &skeleton);
    let spoiler = motion.and_then(|_| skeleton.find(SPOILER_BONE));
    if let Some(bone) = spoiler.filter(|&bone| parts.iter().all(|part| part.bone != bone)) {
        parts.push(HingedPart {
            name: SPOILER_BONE.to_owned(),
            bone,
            zone: REAR_ZONE,
            centre: skeleton.bones[bone].position,
            limits: [[0.0; 2]; 3],
            mass_kg: SPOILER_MASS_KG,
            thresholds: Vec::new(),
        });
    }
    let levels = body_parts(pool, spec, &skeleton, &parts).ok()?;
    let finest = levels.into_iter().next()?;
    let body = vehicle.body.first()?;
    let aligned = finest.len() == body.len() && finest.iter().zip(body).all(|(c, m)| c.len() == m.triangles.len());
    if !aligned {
        log::warn!("the skin of the body does not line up with its meshes: no hinged parts");
        return None;
    }
    let mut counts = vec![0usize; parts.len()];
    finest.iter().flatten().flatten().for_each(|&part| counts[part] += 1);
    let keep: Vec<usize> = (0..parts.len()).filter(|&at| counts[at] > 0 && usable(&parts[at], spoiler)).collect();
    if keep.is_empty() {
        return None;
    }
    let remap = |at: usize| keep.iter().position(|&kept| kept == at);
    let classes = finest.iter().map(|mesh| mesh.iter().map(|class| class.and_then(remap)).collect()).collect();
    let parts = keep
        .into_iter()
        .map(|at| {
            let part = &parts[at];
            Part {
                name: part.name.clone(),
                zone: part.zone,
                pivot: skeleton.bones[part.bone].position,
                limits: part.limits,
                thresholds: part.thresholds.clone(),
                mass_kg: part.mass_kg,
                active: motion.filter(|_| Some(part.bone) == spoiler),
            }
        })
        .collect();
    Some(Hinged { parts, classes })
}

/// A part the game can play: a zone it knows and a range to swing through, or the active spoiler (`spoiler` is its bone).
fn usable(part: &HingedPart, spoiler: Option<usize>) -> bool {
    zones(part.zone).is_some() && (swing(&part.limits).is_some() || Some(part.bone) == spoiler)
}

/// The zones whose damage moves a part of `code`: its own and the corners beside it. The file names one zone per part
/// (1 front, 2 rear, 3 left, 4 right, 5 to 8 the corners front left, front right, rear left, rear right); a corner hit
/// reaches the side parts the corner touches.
pub fn zones(code: u16) -> Option<&'static [&'static str]> {
    let list: &'static [&'static str] = match code {
        1 => &["front", "front_left", "front_right"],
        2 => &["rear", "rear_left", "rear_right"],
        3 => &["left", "front_left", "rear_left"],
        4 => &["right", "front_right", "rear_right"],
        5 => &["front_left"],
        6 => &["front_right"],
        7 => &["rear_left"],
        8 => &["rear_right"],
        _ => return None,
    };
    Some(list)
}

/// The one swing a manifest entry describes: the axis (0 x, 1 y, 2 z of the 2012 space) and the angle of the largest
/// range end, in degrees. The first axis wins a tie, the positive end wins a tie of ends. `None` for a part with no range.
pub fn swing(limits: &[[f32; 2]; 3]) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32)> = None;
    for (axis, [low, high]) in limits.iter().enumerate() {
        let end = if low.abs() > high.abs() { *low } else { *high };
        if end.abs() > best.map_or(0.0, |(_, angle)| angle.abs()) {
            best = Some((axis, end));
        }
    }
    best
}

/// The solid-name stem of a part: `BonnetHinge` is `BONNET`, `FrontLeftDoorHinge` is `FRONT_LEFT_DOOR`.
pub fn stem(name: &str) -> String {
    let name = label(name);
    let mut out = String::new();
    for (at, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() && at > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}

/// The part's name without the word `Hinge`, which stays before a trailing number (`LeftMirrorHinge1` is `LeftMirror1`).
fn label(name: &str) -> String {
    let digits = name.len() - name.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    let (word, number) = name.split_at(name.len() - digits);
    match word.strip_suffix("Hinge").filter(|rest| !rest.is_empty()) {
        Some(rest) => format!("{rest}{number}"),
        None => name.to_owned(),
    }
}

/// A part's solids: the finest body solid `body` cut into the triangles of each part and the rest. Returns the body
/// without the parts and, for each part, its solid named `<stem>_<letter>` (`None` when the cut left it empty).
pub fn split(
    body: &SolidSource,
    stems: &[String],
    letter: char,
    classes: &[Vec<Option<usize>>],
) -> Option<(SolidSource, Vec<Option<SolidSource>>)> {
    // Groups come from the meshes that have triangles, in order.
    let meshes: Vec<&Vec<Option<usize>>> = classes.iter().filter(|mesh| !mesh.is_empty()).collect();
    let aligned = meshes.len() == body.groups.len()
        && body.groups.iter().zip(&meshes).all(|(group, mesh)| group.indices.len() == mesh.len() * 3);
    if !aligned {
        return None;
    }
    let mut rest = Vec::new();
    let mut per_part: Vec<Vec<GroupSource>> = vec![Vec::new(); stems.len()];
    for (group, mesh) in body.groups.iter().zip(meshes) {
        let pick = |wanted: Option<usize>| subset(group, mesh, wanted);
        rest.extend(pick(None));
        for (at, groups) in per_part.iter_mut().enumerate() {
            groups.extend(pick(Some(at)));
        }
    }
    let mut left = body.clone();
    left.groups = rest;
    let (min, max) = bounds(&left.groups).unwrap_or((body.bounds_min, body.bounds_max));
    (left.bounds_min, left.bounds_max) = (min, max);
    let parts = per_part
        .into_iter()
        .zip(stems)
        .map(|(groups, stem)| {
            if groups.is_empty() {
                return None;
            }
            let mut solid = SolidSource::new(format!("{stem}_{letter}"), groups);
            solid.texture_hashes = body.texture_hashes.clone();
            solid.light_material_hashes = body.light_material_hashes.clone();
            Some(solid)
        })
        .collect();
    Some((left, parts))
}

fn bounds(groups: &[GroupSource]) -> Option<([f32; 3], [f32; 3])> {
    let mut points = groups.iter().flat_map(|group| group.vertices.iter()).map(|v| v.position).peekable();
    points.peek()?;
    let (mut low, mut high) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in points {
        for axis in 0..3 {
            low[axis] = low[axis].min(p[axis]);
            high[axis] = high[axis].max(p[axis]);
        }
    }
    Some((low, high))
}

/// The triangles of `group` that belong to `wanted` (`None`: to no part), with the vertices they use.
fn subset(group: &GroupSource, classes: &[Option<usize>], wanted: Option<usize>) -> Option<GroupSource> {
    let mut remap: Vec<Option<u16>> = vec![None; group.vertices.len()];
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices = Vec::new();
    for (triangle, class) in group.indices.as_chunks::<3>().0.iter().zip(classes) {
        if *class != wanted {
            continue;
        }
        for &index in triangle {
            let slot = &mut remap[usize::from(index)];
            let at = *slot.get_or_insert_with(|| {
                vertices.push(group.vertices[usize::from(index)]);
                (vertices.len() - 1) as u16
            });
            indices.push(at);
        }
    }
    if indices.is_empty() {
        return None;
    }
    Some(GroupSource { vertices, indices, bounds: None, ..group.clone() })
}

/// The `[[damage_parts]]` tables for the parts of `hinged`, whose solids are `stems` (the same order), moved into the
/// car's space by `fit`.
pub fn manifest_tables(hinged: &Hinged, stems: &[String], fit: &Fit) -> String {
    let mut text = String::new();
    for (part, stem) in hinged.parts.iter().zip(stems) {
        let Some(zones) = zones(part.zone) else { continue };
        let Some((axis, angle)) = swing(&part.limits).or(part.active.map(|_| (0, 0.0))) else { continue };
        let mut direction = [0.0f32; 3];
        direction[axis] = 1.0;
        let axis = to_car_space(direction);
        let pivot = fit.apply(to_car_space(part.pivot));
        let mut stages: Vec<f32> = part.thresholds.iter().map(|t| t.clamp(0.0, 1.0)).collect();
        stages.sort_by(f32::total_cmp);
        stages.truncate(MAX_STAGES);
        let name = label(&part.name);
        let zone_list: Vec<String> = zones.iter().map(|zone| format!("\"{zone}\"")).collect();
        text.push_str("\n[[damage_parts]]\n");
        text.push_str(&format!("name = \"{name}\"\nmodel = \"{stem}\"\n"));
        text.push_str(&format!("zones = [{}]\n", zone_list.join(", ")));
        text.push_str(&format!("pivot = [{:.5}, {:.5}, {:.5}]\n", pivot[0], pivot[1], pivot[2]));
        text.push_str(&format!("axis = [{:.1}, {:.1}, {:.1}]\nangle = {angle:.2}\n", axis[0], axis[1], axis[2]));
        text.push_str(&format!("mass_kg = {:.1}\n", part.mass_kg.clamp(0.5, 5000.0)));
        if !stages.is_empty() {
            let list: Vec<String> = stages.iter().map(|s| format!("{s:.4}")).collect();
            text.push_str(&format!("stages = [{}]\n", list.join(", ")));
        }
        if let Some(motion) = part.active {
            text.push_str(&active_table(&motion, fit));
        }
    }
    text
}

/// The `[damage_parts.active]` table of a spoiler: its slide in the car's space (the fit scales it with the body), its
/// turn about the pivot (the cyclic change of axes keeps the handedness, so the axis and the angle carry over), the
/// speeds and the time. All of them are what the car's data says.
fn active_table(motion: &SpoilerMotion, fit: &Fit) -> String {
    let moved = to_car_space(motion.translation);
    let offset = [0, 1, 2].map(|k| moved[k] * fit.scale[k]);
    let mut text = String::from("\n[damage_parts.active]\n");
    if offset.iter().any(|c| c.abs() >= 1e-4) {
        text.push_str(&format!("offset = [{:.5}, {:.5}, {:.5}]\n", offset[0], offset[1], offset[2]));
    }
    if let Some((axis, degrees)) = motion.rotation_axis_angle() {
        let axis = to_car_space(axis);
        text.push_str(&format!(
            "rotation_axis = [{:.5}, {:.5}, {:.5}]\nrotation_deg = {degrees:.3}\n",
            axis[0], axis[1], axis[2]
        ));
    }
    text.push_str(&format!(
        "above_mph = {:.1}\nbelow_mph = {:.1}\nseconds = {:.3}\n",
        motion.above_mph, motion.below_mph, motion.seconds
    ));
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(x: f32) -> Vertex {
        Vertex { position: [x, 0.0, 0.0], normal: [0.0, 0.0, 1.0], color_bgra: [255; 4], uv: [x, 0.0] }
    }

    fn body() -> SolidSource {
        // One group of four triangles on five vertices; the middle two belong to part 0 and part 1.
        let vertices = (0..5).map(|n| vertex(n as f32)).collect();
        let indices = vec![0, 1, 2, 1, 2, 3, 2, 3, 4, 0, 3, 4];
        let mut group = GroupSource::new(vertices, indices);
        group.flags = 7;
        let mut solid = SolidSource::new("CAR_KIT00_BODY_A", vec![group]);
        solid.texture_hashes = vec![5];
        solid
    }

    #[test]
    fn a_cut_keeps_every_triangle_exactly_once() {
        let classes = vec![vec![None, Some(0), Some(1), None]];
        let stems = vec!["CAR_KIT00_BONNET".to_owned(), "CAR_KIT00_DOOR".to_owned()];
        let (rest, parts) = split(&body(), &stems, 'A', &classes).unwrap();
        let triangles = |solid: &SolidSource| solid.groups.iter().map(|g| g.indices.len() / 3).sum::<usize>();
        assert_eq!(triangles(&rest), 2);
        let bonnet = parts[0].as_ref().unwrap();
        assert_eq!((bonnet.name.as_str(), triangles(bonnet)), ("CAR_KIT00_BONNET_A", 1));
        assert_eq!(triangles(parts[1].as_ref().unwrap()), 1);
        let group = &bonnet.groups[0];
        assert_eq!(group.flags, 7, "the material of the group is kept");
        assert_eq!(bonnet.texture_hashes, vec![5]);
        let xs: Vec<f32> = group.indices.iter().map(|&i| group.vertices[usize::from(i)].position[0]).collect();
        assert_eq!(xs, vec![1.0, 2.0, 3.0], "the triangle keeps its corners");
        assert_eq!(group.vertices.len(), 3, "only the vertices it uses");
    }

    #[test]
    fn a_part_that_owns_nothing_has_no_solid_and_a_misaligned_cut_is_refused() {
        let stems = vec!["A".to_owned(), "B".to_owned()];
        let (_, parts) = split(&body(), &stems, 'B', &[vec![None, Some(0), None, None]]).unwrap();
        assert!(parts[0].is_some() && parts[1].is_none());
        assert!(split(&body(), &stems, 'A', &[vec![None; 3]]).is_none());
        assert!(split(&body(), &stems, 'A', &[]).is_none());
    }

    #[test]
    fn part_names_become_solid_stems() {
        assert_eq!(stem("BonnetHinge"), "BONNET");
        assert_eq!(stem("FrontLeftDoorHinge"), "FRONT_LEFT_DOOR");
        assert_eq!(stem("LeftMirrorHinge1"), "LEFT_MIRROR1");
        assert_eq!(stem("Hinge"), "HINGE");
    }

    #[test]
    fn the_swing_is_the_largest_end_of_the_widest_axis() {
        // The 911's bonnet turns about x, to -15; its left mirror about y, to 15; its doors about z, to 1.
        assert_eq!(swing(&[[-15.0, 0.0], [0.0, 0.0], [0.0, 0.0]]), Some((0, -15.0)));
        assert_eq!(swing(&[[0.0, 0.0], [-5.0, 15.0], [-15.0, 15.0]]), Some((1, 15.0)));
        assert_eq!(swing(&[[0.0, 0.0], [-15.0, 5.0], [-15.0, 15.0]]), Some((1, -15.0)));
        assert_eq!(swing(&[[0.0, 0.0], [0.0, 0.0], [-1.0, 1.0]]), Some((2, 1.0)));
        assert_eq!(swing(&[[0.0; 2]; 3]), None);
    }

    #[test]
    fn corner_zones_reach_the_parts_beside_them() {
        assert_eq!(zones(1), Some(&["front", "front_left", "front_right"][..]));
        assert_eq!(zones(5), Some(&["front_left"][..]));
        assert_eq!(zones(0), None);
        assert_eq!(zones(9), None);
    }

    #[test]
    fn the_manifest_table_carries_the_swing_in_car_space() {
        use crate::fit::Layout;
        let layout = Layout { wheelbase: 2.6, x_mid: 0.0, track: 0.8, hub_z: 0.35, radius: 0.35 };
        let part = Part {
            name: "BonnetHinge".into(),
            zone: 1,
            pivot: [-0.7, 0.88, 0.74],
            limits: [[-15.0, 0.0], [0.0; 2], [0.0; 2]],
            thresholds: vec![0.5, 0.04],
            mass_kg: 50.0,
            active: None,
        };
        let hinged = Hinged { parts: vec![part], classes: Vec::new() };
        let text = manifest_tables(&hinged, &["CAR_KIT00_BONNET".into()], &Fit::own(layout));
        let table: toml::Table = toml::from_str(&text).unwrap();
        let entry = &table["damage_parts"][0];
        assert_eq!(entry["name"].as_str(), Some("Bonnet"));
        assert_eq!(entry["angle"].as_float(), Some(-15.0));
        // The lateral axis (2012 x, to the left) is the game's y.
        assert_eq!(entry["axis"][1].as_float(), Some(1.0));
        assert_eq!(entry["pivot"][0].as_float(), Some(0.74), "forward is x");
        assert_eq!(entry["stages"][0].as_float(), Some(0.04), "stages are sorted");
        assert_eq!(entry["mass_kg"].as_float(), Some(50.0), "the part's own mass is the mass of what falls off");
        assert!(entry.get("detach_at").is_none(), "the game's default ladder rule decides when it falls off");
    }

    #[test]
    fn an_active_spoiler_writes_its_movement_and_needs_no_damage_swing() {
        use crate::fit::Layout;
        let layout = Layout { wheelbase: 2.6, x_mid: 0.0, track: 0.8, hub_z: 0.35, radius: 0.35 };
        let half = 15f32.to_radians();
        let motion = SpoilerMotion {
            bone: 3,
            translation: [0.0, 0.02, -0.07],
            rotation: [half.sin(), 0.0, 0.0, half.cos()],
            seconds: 0.612,
            above_mph: 70.0,
            below_mph: 40.0,
        };
        let part = Part {
            name: "SpoilerHinge".into(),
            zone: 2,
            pivot: [0.0, 0.78, -1.83],
            limits: [[0.0; 2]; 3],
            thresholds: Vec::new(),
            mass_kg: 30.0,
            active: Some(motion),
        };
        let hinged = Hinged { parts: vec![part.clone()], classes: Vec::new() };
        let text = manifest_tables(&hinged, &["CAR_KIT00_SPOILER".into()], &Fit::own(layout));
        let table: toml::Table = toml::from_str(&text).unwrap();
        let entry = &table["damage_parts"][0];
        assert_eq!((entry["name"].as_str(), entry["angle"].as_float()), (Some("Spoiler"), Some(0.0)));
        let active = &entry["active"];
        // The engine's up (+y) is the game's up (+z); back (-z) is -x.
        assert_eq!(active["offset"][0].as_float(), Some(-0.07));
        assert_eq!(active["offset"][2].as_float(), Some(0.02));
        // The lateral axis (2012 x) is the game's y; the angle is kept.
        assert_eq!(active["rotation_axis"][1].as_float(), Some(1.0));
        assert!((active["rotation_deg"].as_float().unwrap() - 30.0).abs() < 1e-3);
        assert_eq!((active["above_mph"].as_float(), active["below_mph"].as_float()), (Some(70.0), Some(40.0)));
        assert_eq!(active["seconds"].as_float(), Some(0.612));
        // Without a movement, a part with no swing is not written at all.
        let still = Hinged { parts: vec![Part { active: None, ..part }], classes: Vec::new() };
        assert!(manifest_tables(&still, &["CAR_KIT00_SPOILER".into()], &Fit::own(layout)).is_empty());
    }

    #[test]
    #[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
    fn the_911_has_hinged_parts_and_an_active_spoiler() {
        let Some(dir) = std::env::var_os("MW2012_GAME_DIR") else { return };
        let source = crate::source::Source::open(std::path::Path::new(&dir)).unwrap();
        let file = source.cars().iter().find(|car| car.number == 1085007).unwrap();
        let car = source.load(file).unwrap();
        let hinged = car.hinged.as_ref().expect("the 911 has hinged parts");
        let names: Vec<&str> = hinged.parts.iter().map(|part| part.name.as_str()).collect();
        assert_eq!(hinged.parts.len(), 8, "{names:?}");
        let spoiler = hinged.parts.iter().find(|part| part.name == "SpoilerHinge").expect("the spoiler");
        let motion = spoiler.active.expect("it moves");
        assert_eq!((motion.above_mph, motion.below_mph), (70.0, 40.0));
        assert!(hinged.parts.iter().filter(|part| part.active.is_some()).count() == 1);
    }
}
