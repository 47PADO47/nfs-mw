//! The hinged parts of a mod car (the `[[damage_parts]]` tables of the car manifest, docs/modding/car-damage-parts.md).
//!
//! A hinged part is a solid of the car's own `GEOMETRY.BIN` that the body does not contain. While the car is intact the
//! part is drawn where it was modelled; as the damage of the zones around it grows it turns about its hinge line, and
//! past `detach_at` it is gone. [`add`] puts the parts on an assembled car as placements of their own slots, and
//! [`Hinge::pose`] says how far each has turned for the zone levels of the car's damage.

use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

use blackbox_hash::bstring_hash;
use blackbox_solid::Solid;
use glam::{Mat4, Vec3};

use crate::car::{LodAssembly, Placement};

/// The first slot number of the hinged parts: part `n` of a car is slot `HINGED_SLOT + n`. The slots of the parts
/// database end at 135, so these never meet one.
pub const HINGED_SLOT: usize = 256;
/// The zone levels the damage model holds, 0 to this.
pub const MAX_LEVEL: u8 = 3;
/// The first number of the zone levels carried in the set of revealed slots (`level_mark`): well above every slot.
const LEVEL_BASE: usize = 1024;
/// Levels of detail a car has, `A` to `E`.
const LEVELS: usize = 5;

/// One hinged part, as the car's manifest describes it.
#[derive(Debug, Clone, PartialEq)]
pub struct Hinge {
    pub name: String,
    /// The part's solid without the level-of-detail letter.
    pub model: String,
    /// The zones (in the order of `physics::DAMAGE_ZONES`, 0 to 7) whose damage moves the part.
    pub zones: Vec<usize>,
    /// A point of the hinge line and its unit direction, in the car's solid space.
    pub pivot: [f32; 3],
    pub axis: [f32; 3],
    /// Degrees turned at the last stage.
    pub angle: f32,
    /// The zone damage fractions at which the stages start, ascending.
    pub stages: Vec<f32>,
    pub detach_at: Option<f32>,
}

/// How a hinged part stands for some damage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pose {
    /// Turned `degrees` about its hinge (0 while the car is intact in its zones).
    Turned(f32),
    /// Fallen off: not drawn.
    Gone,
}

impl Hinge {
    /// The part's damage fraction: the worst panel level of its zones over [`MAX_LEVEL`]. `level(zone)` gives a zone's
    /// panel level.
    pub fn fraction(&self, level: impl Fn(usize) -> u8) -> f32 {
        let worst = self.zones.iter().map(|&zone| level(zone).min(MAX_LEVEL)).max().unwrap_or(0);
        f32::from(worst) / f32::from(MAX_LEVEL)
    }

    /// The part's pose at damage `fraction` (0 to 1). No damage leaves it at rest; otherwise it has turned `angle` times
    /// the share of its stages that the damage has reached.
    pub fn pose(&self, fraction: f32) -> Pose {
        if fraction <= 0.0 {
            return Pose::Turned(0.0);
        }
        if self.detach_at.is_some_and(|at| fraction >= at) {
            return Pose::Gone;
        }
        let reached = self.stages.iter().filter(|&&stage| fraction >= stage).count();
        Pose::Turned(self.angle * reached as f32 / self.stages.len().max(1) as f32)
    }

    /// The transform of the part turned by `degrees` about its hinge line, in the car's solid space.
    pub fn turn(&self, degrees: f32) -> Mat4 {
        let pivot = Vec3::from_array(self.pivot);
        let axis = Vec3::from_array(self.axis);
        if degrees == 0.0 || axis.length_squared() < 1e-12 {
            return Mat4::IDENTITY;
        }
        Mat4::from_translation(pivot)
            * Mat4::from_axis_angle(axis.normalize(), degrees.to_radians())
            * Mat4::from_translation(-pivot)
    }
}

static HINGES: RwLock<Option<HashMap<String, Arc<Vec<Hinge>>>>> = RwLock::new(None);

/// Publish the hinged parts of the car in `folder`, as `customization` does for the shop. An empty list publishes nothing.
pub fn publish(folder: &str, hinges: Vec<Hinge>) {
    if hinges.is_empty() {
        return;
    }
    let mut all = HINGES.write().unwrap_or_else(PoisonError::into_inner);
    all.get_or_insert_with(HashMap::new).insert(folder.to_ascii_uppercase(), Arc::new(hinges));
}

/// The hinged parts of the mod car in `folder`. `None` for a car with none.
pub fn of(folder: &str) -> Option<Arc<Vec<Hinge>>> {
    let all = HINGES.read().unwrap_or_else(PoisonError::into_inner);
    all.as_ref()?.get(&folder.to_ascii_uppercase()).cloned()
}

/// The mark that says zone `zone` has panel `level` (1 to [`MAX_LEVEL`]). The marks travel in the set of revealed slots
/// that every car draw already takes (`CarRig::instances_detail`), so the hinged parts need no signature of their own.
pub fn level_mark(zone: usize, level: u8) -> usize {
    LEVEL_BASE + zone * 8 + usize::from(level.min(MAX_LEVEL))
}

/// The panel level of `zone` that `marks` (the revealed slots) carry: the highest mark, 0 when there is none.
pub fn level_in(marks: &std::collections::BTreeSet<usize>, zone: usize) -> u8 {
    (1..=MAX_LEVEL).rev().find(|&level| marks.contains(&level_mark(zone, level))).unwrap_or(0)
}

/// The slot of part `n`.
pub fn slot_of(n: usize) -> usize {
    HINGED_SLOT + n
}

/// The index of the hinged part whose slot is `slot`.
pub fn index_of(slot: usize) -> Option<usize> {
    slot.checked_sub(HINGED_SLOT)
}

/// The solid of `model` at `level`. A part has solids only at the levels where the car's body does not hold it, so a
/// level without one draws the part as the body.
fn solid_at(pool: &HashMap<u32, Arc<Solid>>, model: &str, level: usize) -> Option<u32> {
    let hash = bstring_hash(&format!("{model}_{}", char::from(b'A' + level.min(LEVELS - 1) as u8)));
    pool.contains_key(&hash).then_some(hash)
}

/// Add the hinged parts of the car in `folder` to its placements: every part at each level of detail it has a solid for
/// (`placements` is level `lod`, `levels` the driving levels). A part with no solid at any level is left out and the log
/// says so. Returns how many parts were added.
pub fn add(
    folder: &str,
    lod: usize,
    pool: &HashMap<u32, Arc<Solid>>,
    placements: &mut Vec<Placement>,
    levels: &mut [LodAssembly],
) -> usize {
    let Some(hinges) = of(folder) else { return 0 };
    let mut added = 0;
    for (n, hinge) in hinges.iter().enumerate() {
        let place = |level: usize| {
            solid_at(pool, &hinge.model, level).map(|solid| Placement {
                solid,
                transform: Mat4::IDENTITY,
                left_brake: false,
                slot: slot_of(n),
                corner: None,
            })
        };
        let mut found = place(lod).into_iter().inspect(|p| placements.push(p.clone())).count();
        for assembly in levels.iter_mut() {
            let level = assembly.level;
            found += place(level).into_iter().inspect(|p| assembly.placements.push(p.clone())).count();
        }
        if found == 0 && (0..LEVELS).all(|level| place(level).is_none()) {
            log::warn!("{folder}: the hinged part {} has no solid named {}_A..E: not added", hinge.name, hinge.model);
            continue;
        }
        added += 1;
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bonnet() -> Hinge {
        Hinge {
            name: "Bonnet".into(),
            model: "CAR_KIT00_BONNET".into(),
            zones: vec![0, 4, 5],
            pivot: [0.74, -0.7, 0.88],
            axis: [0.0, 1.0, 0.0],
            angle: -15.0,
            stages: vec![0.04, 0.05, 0.5],
            detach_at: None,
        }
    }

    #[test]
    fn an_undamaged_part_stays_at_rest_and_each_stage_turns_it_further() {
        let hinge = bonnet();
        assert_eq!(hinge.pose(0.0), Pose::Turned(0.0));
        // One panel level of three is 0.33: the first two stages are passed.
        assert_eq!(hinge.pose(1.0 / 3.0), Pose::Turned(-10.0));
        assert_eq!(hinge.pose(2.0 / 3.0), Pose::Turned(-15.0));
        assert_eq!(hinge.pose(0.045), Pose::Turned(-5.0));
    }

    #[test]
    fn a_part_with_a_detach_level_falls_off_there() {
        let hinge = Hinge { detach_at: Some(0.9), ..bonnet() };
        assert_eq!(hinge.pose(2.0 / 3.0), Pose::Turned(-15.0));
        assert_eq!(hinge.pose(1.0), Pose::Gone);
    }

    #[test]
    fn the_worst_zone_decides() {
        let hinge = bonnet();
        let levels = [0u8, 0, 0, 0, 2, 3, 0, 0];
        assert_eq!(hinge.fraction(|zone| levels[zone]), 1.0);
        assert_eq!(hinge.fraction(|_| 0), 0.0);
        assert_eq!(hinge.fraction(|_| 9), 1.0, "levels past the top count as the top");
    }

    #[test]
    fn a_turn_keeps_the_pivot_and_swings_the_rest() {
        let hinge = Hinge { pivot: [1.0, 0.0, 0.0], axis: [0.0, 0.0, 1.0], ..bonnet() };
        let turn = hinge.turn(90.0);
        assert!((turn.transform_point3(Vec3::new(1.0, 0.0, 0.0)) - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5);
        assert!((turn.transform_point3(Vec3::new(2.0, 0.0, 0.0)) - Vec3::new(1.0, 1.0, 0.0)).length() < 1e-5);
        assert_eq!(hinge.turn(0.0), Mat4::IDENTITY);
    }

    #[test]
    fn parts_are_published_per_folder_and_slots_are_clear_of_the_database() {
        publish("HINGETEST", vec![bonnet()]);
        assert_eq!(of("hingetest").unwrap().len(), 1);
        assert!(of("OTHERCAR").is_none());
        publish("EMPTYTEST", Vec::new());
        assert!(of("EMPTYTEST").is_none());
        assert_eq!(index_of(slot_of(3)), Some(3));
        let marks: std::collections::BTreeSet<usize> = [level_mark(4, 1), level_mark(4, 2), level_mark(0, 3)].into();
        assert_eq!((level_in(&marks, 4), level_in(&marks, 0), level_in(&marks, 1)), (2, 3, 0));
        assert!(marks.iter().all(|&mark| mark > crate::car::slot::HUD_COLOURS.end().to_owned() + 256 * 3));
        assert_eq!(index_of(10), None);
        assert!(HINGED_SLOT > *crate::car::slot::HUD_COLOURS.end());
    }
}
