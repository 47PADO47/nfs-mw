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
use chameleon_vehicle::{
    HingedPart, Skeleton, SpoilerMotion, Vehicle, attached_motions, body_parts, hinged_parts, spoiler_motion,
};

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
    /// The name of the part it hangs from (the spoiler, for a flap the spoiler's animation moves).
    pub parent: Option<String>,
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
    // The spoiler and the bones that hang from it and move with it, each with the motion it plays.
    let mut moving: Vec<(usize, SpoilerMotion, Option<usize>)> = Vec::new();
    if let (Some(motion), Some(bone)) = (motion, spoiler) {
        moving.push((bone, motion, None));
        for attached in attached_motions(bundle, &skeleton) {
            let own = SpoilerMotion {
                bone: attached.bone,
                translation: attached.translation,
                rotation: attached.rotation,
                ..motion
            };
            moving.push((attached.bone, own, Some(attached.parent)));
        }
    }
    for (at, (bone, _, _)) in moving.iter().enumerate() {
        if parts.iter().any(|part| part.bone == *bone) {
            continue;
        }
        parts.push(HingedPart {
            name: if at == 0 { SPOILER_BONE.to_owned() } else { attached_name(&moving[at].1, at) },
            bone: *bone,
            zone: REAR_ZONE,
            centre: skeleton.bones[*bone].position,
            limits: [[0.0; 2]; 3],
            mass_kg: SPOILER_MASS_KG,
            thresholds: Vec::new(),
        });
    }
    let active_bones: Vec<usize> = moving.iter().map(|(bone, _, _)| *bone).collect();
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
    let keep: Vec<usize> = (0..parts.len()).filter(|&at| counts[at] > 0 && usable(&parts[at], &active_bones)).collect();
    if keep.is_empty() {
        return None;
    }
    let remap = |at: usize| keep.iter().position(|&kept| kept == at);
    let classes = finest.iter().map(|mesh| mesh.iter().map(|class| class.and_then(remap)).collect()).collect();
    let parts = keep
        .into_iter()
        .map(|at| {
            let part = &parts[at];
            let moves = moving.iter().find(|(bone, _, _)| *bone == part.bone);
            let parent =
                moves.and_then(|(_, _, parent)| *parent).and_then(|bone| parts.iter().find(|known| known.bone == bone));
            Part {
                name: part.name.clone(),
                zone: part.zone,
                pivot: skeleton.bones[part.bone].position,
                limits: part.limits,
                thresholds: part.thresholds.clone(),
                mass_kg: part.mass_kg,
                active: moves.map(|(_, motion, _)| *motion),
                parent: parent.map(|known| label(&known.name)),
            }
        })
        .collect();
    Some(Hinged { parts, classes })
}

/// A part the game can play: a zone it knows and a range to swing through, or the active spoiler (`spoiler` is its bone).
fn usable(part: &HingedPart, active_bones: &[usize]) -> bool {
    zones(part.zone).is_some() && (swing(&part.limits).is_some() || active_bones.contains(&part.bone))
}

/// The name of a flap the spoiler's animation moves: which side it moves to (the engine's x is to the left), else its number.
fn attached_name(motion: &SpoilerMotion, number: usize) -> String {
    let [x, y, z] = motion.translation;
    match (x.abs() > y.abs().max(z.abs()), x > 0.0) {
        (true, true) => "SpoilerFlapLeft".to_owned(),
        (true, false) => "SpoilerFlapRight".to_owned(),
        (false, _) => format!("SpoilerFlap{number}"),
    }
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
        if let Some(parent) = &part.parent {
            text.push_str(&format!("parent = \"{parent}\"\n"));
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
mod tests;
