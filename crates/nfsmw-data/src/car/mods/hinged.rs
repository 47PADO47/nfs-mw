//! The hinged parts of a mod car (the `[[damage_parts]]` tables of the car manifest, docs/modding/car-damage-parts.md).
//!
//! A hinged part is a solid of the car's own `GEOMETRY.BIN` that the body does not contain. While the car is intact the
//! part is drawn where it was modelled; as the damage of the zones around it grows it turns about its hinge line, and
//! past its release level it leaves the car ([`debris`]). [`add`] puts the parts on an assembled car as placements of their own slots, and
//! [`Hinge::pose`] says how far each has turned for the zone levels of the car's damage.

use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

use blackbox_hash::bstring_hash;
use blackbox_solid::Solid;
use glam::{Mat4, Vec3};

use crate::car::{LodAssembly, Placement};

mod active;
mod debris;
mod drive;
mod steering;
mod wing;
pub use active::{Active, Deployment, MPS_TO_MPH, deploy_in, deploy_mark};
pub use debris::{DEBRIS_SOUND, Motion, Parent, debris_bounds, debris_dynamics, debris_motion, part_axis, part_frame};
pub use drive::{
    Drive, DriveState, Input as DriveInput, Inputs as DriveInputs, MAX_DRIVES, Op as DriveOp, Test as DriveTest,
};
pub use steering::Steering;
pub use wing::{Blend, SLOTS, Sample, Wing, WingState, WingTrack};

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
    /// The degrees turned at each stage (one per stage); empty: `angle` times the share of stages reached.
    pub stage_angles: Vec<f32>,
    pub detach_at: Option<f32>,
    /// Whether the part can leave the car at all.
    pub detach: bool,
    /// What the part weighs as a body of its own, in kg.
    pub mass_kg: f32,
    /// How the part moves with the car's speed (an active spoiler), if it does.
    pub active: Option<Active>,
    /// The movements of the parts it hangs from, outermost first: they carry the part along with the same deployment (or the
    /// same wing blend) before its own movement.
    pub ancestors: Vec<Ancestor>,
    /// The keyed animations that move the part, when a wing machine does (`[wing]`).
    pub tracks: Vec<WingTrack>,
    /// The conditions that play the part's active movement (`[[damage_parts.drive]]`).
    pub drives: Vec<Drive>,
    /// The wing machine of the car (the same on every part it moves).
    pub wing: Option<Wing>,
    /// How the steering turns the part, for a steering wheel (`[damage_parts.steering]`).
    pub steering: Option<Steering>,
}

/// A part a hinged part hangs from, as far as its movement goes.
#[derive(Debug, Clone, PartialEq)]
pub struct Ancestor {
    pub active: Option<Active>,
    pub tracks: Vec<WingTrack>,
    pub pivot: [f32; 3],
}

/// How a hinged part stands for some damage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pose {
    /// Turned `degrees` about its hinge (0 while the car is intact in its zones).
    Turned(f32),
    /// Released: the car no longer draws it (it is a body of its own, see [`debris`]).
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
        if self.releases(fraction, false) {
            return Pose::Gone;
        }
        Pose::Turned(self.swing(fraction))
    }

    /// Degrees the part has turned at damage `fraction`, whether or not it has been released since: 0 without damage,
    /// else `angle` times the share of the stages the damage has reached.
    pub fn swing(&self, fraction: f32) -> f32 {
        if fraction <= 0.0 {
            return 0.0;
        }
        let reached = self.stages.iter().filter(|&&stage| fraction >= stage).count();
        if self.stage_angles.len() == self.stages.len() {
            return reached.checked_sub(1).map_or(0.0, |last| self.stage_angles[last]);
        }
        self.angle * reached as f32 / self.stages.len().max(1) as f32
    }

    /// The damage fraction from which the part leaves the car: the manifest's `detach_at`, else the last stage but not
    /// before half the damage a zone can take. `None` for a part that stays on the car.
    pub fn release_level(&self) -> Option<f32> {
        if !self.detach {
            return None;
        }
        Some(self.detach_at.unwrap_or_else(|| self.stages.last().copied().unwrap_or(0.0).max(0.5)))
    }

    /// Whether the part is to leave the car now: its damage has reached the release level, or the car is `wrecked`
    /// (destroyed or busted) and the part can detach at all.
    pub fn releases(&self, fraction: f32, wrecked: bool) -> bool {
        let Some(level) = self.release_level() else { return false };
        wrecked || (fraction > 0.0 && fraction >= level)
    }

    /// Where the part stands in the car's solid space: moved by its active movement at `amount` of its deployment, then
    /// turned by `degrees` about its hinge line.
    pub fn transform(&self, degrees: f32, amount: f32) -> Mat4 {
        self.transform_with(degrees, amount, None)
    }

    /// [`Hinge::transform`] with the wing machine's `blend`, for a part that its animations move.
    pub fn transform_with(&self, degrees: f32, amount: f32, blend: Option<&Blend>) -> Mat4 {
        let motion = |active: Option<&Active>, tracks: &[WingTrack], pivot: [f32; 3]| {
            let pivot = Vec3::from_array(pivot);
            match (active, blend) {
                (Some(active), _) => active.motion(pivot, amount),
                (None, Some(blend)) if !tracks.is_empty() => blend.pose(tracks).matrix(pivot),
                _ => Mat4::IDENTITY,
            }
        };
        let carried: Mat4 = self
            .ancestors
            .iter()
            .fold(Mat4::IDENTITY, |chain, up| chain * motion(up.active.as_ref(), &up.tracks, up.pivot));
        self.turn(degrees) * carried * motion(self.active.as_ref(), &self.tracks, self.pivot)
    }

    /// The turn the steering gives the part at `steering` (-1 to 1, positive to the right): the identity for a part the steering
    /// does not turn. It is applied to the part where it was modelled, before the damage and the speed move it.
    pub fn steered(&self, steering: f32) -> Mat4 {
        let Some(wheel) = self.steering else { return Mat4::IDENTITY };
        self.turn(wheel.angle(steering))
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
            stage_angles: Vec::new(),
            detach_at: None,
            detach: false,
            mass_kg: 40.0,
            active: None,
            ancestors: Vec::new(),
            tracks: Vec::new(),
            drives: Vec::new(),
            wing: None,
            steering: None,
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
    fn stage_angles_name_the_turn_of_each_stage() {
        let hinge = Hinge { stage_angles: vec![0.0, -2.0, -9.0], ..bonnet() };
        assert_eq!(hinge.swing(0.0), 0.0);
        assert_eq!(hinge.swing(0.045), 0.0, "the first stage turns it by its own 0");
        assert_eq!(hinge.swing(1.0 / 3.0), -2.0);
        assert_eq!(hinge.swing(1.0), -9.0);
        let even = Hinge { stage_angles: vec![1.0], ..bonnet() };
        assert_eq!(even.swing(1.0), -15.0, "a list of another length is ignored");
    }

    #[test]
    fn a_part_with_a_detach_level_falls_off_there() {
        let hinge = Hinge { detach: true, detach_at: Some(0.9), ..bonnet() };
        assert_eq!(hinge.pose(2.0 / 3.0), Pose::Turned(-15.0));
        assert_eq!(hinge.pose(1.0), Pose::Gone);
    }

    #[test]
    fn without_a_detach_level_the_last_stage_decides_but_not_before_half_the_damage() {
        let bonnet = Hinge { detach: true, ..bonnet() };
        assert_eq!(bonnet.release_level(), Some(0.5), "the last stage of the 911's bonnet");
        let mirror = Hinge { stages: vec![0.0, 0.09, 0.2], ..bonnet.clone() };
        assert_eq!(mirror.release_level(), Some(0.5), "a mirror's 0.2 is the first dent: not that early");
        let door = Hinge { stages: vec![0.4, 1.0], ..bonnet.clone() };
        assert_eq!(door.release_level(), Some(1.0));
        assert!(!door.releases(2.0 / 3.0, false) && door.releases(1.0, false));
        assert!(bonnet.releases(2.0 / 3.0, false) && !bonnet.releases(1.0 / 3.0, false));
        assert_eq!(bonnet.pose(2.0 / 3.0), Pose::Gone);
        assert_eq!(bonnet.swing(2.0 / 3.0), -15.0, "the swing it had when it left");
    }

    #[test]
    fn a_wreck_sheds_every_part_that_can_detach_and_only_those() {
        let free = Hinge { detach: true, ..bonnet() };
        assert!(free.releases(0.0, true), "an undamaged zone does not save a wrecked car's part");
        assert!(!free.releases(0.0, false));
        let kept = Hinge { detach: false, ..bonnet() };
        assert!(!kept.releases(1.0, true) && !kept.releases(1.0, false));
        assert_eq!(kept.release_level(), None);
        let set = Hinge { detach: true, detach_at: Some(0.2), ..bonnet() };
        assert!(set.releases(1.0 / 3.0, false), "an explicit level is used as it is");
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
    fn an_active_part_is_moved_before_it_is_turned() {
        let active = Active {
            offset: [0.0, 0.0, 1.0],
            rotation: None,
            above_mph: 70.0,
            below_mph: 40.0,
            seconds: 0.6,
            gas: false,
            delay: None,
        };
        let hinge = Hinge { pivot: [1.0, 0.0, 0.0], axis: [0.0, 0.0, 1.0], active: Some(active), ..bonnet() };
        assert_eq!(hinge.transform(0.0, 0.0), Mat4::IDENTITY);
        let up = hinge.transform(0.0, 1.0).transform_point3(Vec3::ZERO);
        assert!((up - Vec3::Z).length() < 1e-6, "moved up a metre");
        let both = hinge.transform(90.0, 1.0).transform_point3(Vec3::new(2.0, 0.0, 0.0));
        assert!((both - Vec3::new(1.0, 1.0, 1.0)).length() < 1e-5, "moved, then turned about the hinge: {both:?}");
        assert_eq!(bonnet().transform(0.0, 1.0), Mat4::IDENTITY, "a part with no movement ignores the deployment");
    }

    #[test]
    fn a_part_rides_on_the_movement_of_the_part_it_hangs_from() {
        let wing = Active {
            offset: [0.0, 0.0, 1.0],
            rotation: None,
            above_mph: 70.0,
            below_mph: 40.0,
            seconds: 0.6,
            gas: false,
            delay: None,
        };
        let flap = Active { offset: [0.0, 0.5, 0.0], ..wing };
        let hinge = Hinge {
            active: Some(flap),
            ancestors: vec![Ancestor { active: Some(wing), tracks: Vec::new(), pivot: [0.0; 3] }],
            ..bonnet()
        };
        let moved = hinge.transform(0.0, 1.0).transform_point3(Vec3::ZERO);
        assert!((moved - Vec3::new(0.0, 0.5, 1.0)).length() < 1e-6, "the wing's lift and the flap's slide: {moved:?}");
        let half = hinge.transform(0.0, 0.5).transform_point3(Vec3::ZERO);
        assert!((half - Vec3::new(0.0, 0.25, 0.5)).length() < 1e-6, "both in proportion: {half:?}");
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
