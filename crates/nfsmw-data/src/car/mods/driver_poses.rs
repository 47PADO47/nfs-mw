//! A mod car's driver in several poses (docs/modding/car-customization.md, "The driver's poses").
//!
//! The game draws a car's driver as one static solid, `<FOLDER>_KIT00_DRIVER_<A..D>`. A mod car may also ship the driver
//! posed for the steering: `<FOLDER>_KIT00_DRIVER_S<k>_<A..D>` for k = 0, 1, 2, ... (k = 0 full left, the last full right, the
//! middle the wheel straight) and `<FOLDER>_KIT00_DRIVER_BACK_<A..D>`, the driver looking back for the reverse gear. Each
//! pose is a placement of its own in a slot of [`POSE_SLOT`] and above; the renderer draws only the one the steering input is
//! nearest to ([`slot_for`]). At a level of detail that has poses the plain driver is not placed; at a level without, it is.
//! A car with no pose solids is unchanged.

use std::collections::HashMap;
use std::sync::Arc;

use blackbox_hash::bstring_hash;
use blackbox_solid::Solid;
use glam::Mat4;

use crate::car::tables::slot;
use crate::car::{LodAssembly, Placement};

/// The slot of steering pose 0. The slots of the parts database end at 135 and the hinged parts start at 256
/// ([`super::hinged::HINGED_SLOT`]), so the poses sit above both.
pub const POSE_SLOT: usize = 512;
/// The most steering poses a car may have.
pub const MAX_STEER_POSES: usize = 31;
/// The slot of the look-back pose.
pub const BACK_SLOT: usize = POSE_SLOT + MAX_STEER_POSES;
/// Levels of detail a car has, `A` to `E`.
const LEVELS: usize = 5;

/// The slot of steering pose `k`.
pub fn steer_slot(k: usize) -> usize {
    POSE_SLOT + k
}

/// Whether `slot` holds a pose of the driver.
pub fn is_pose(slot: usize) -> bool {
    (POSE_SLOT..=BACK_SLOT).contains(&slot)
}

/// How many steering poses `placements` hold (the poses are contiguous from 0).
pub fn steer_count(placements: &[Placement]) -> usize {
    placements
        .iter()
        .filter(|p| (POSE_SLOT..BACK_SLOT).contains(&p.slot))
        .map(|p| p.slot - POSE_SLOT + 1)
        .max()
        .unwrap_or(0)
}

/// The slot of the pose to draw: the look back when the car reverses and has one, else the steering pose nearest to
/// `steering` (-1 full left to 1 full right) among `count`. `None` when the car has no steering pose.
pub fn slot_for(steering: f32, reverse: bool, count: usize, has_back: bool) -> Option<usize> {
    if reverse && has_back {
        return Some(BACK_SLOT);
    }
    if count == 0 {
        return None;
    }
    let steering = if steering.is_finite() { steering.clamp(-1.0, 1.0) } else { 0.0 };
    let nearest = ((steering * 0.5 + 0.5) * (count - 1) as f32).round() as usize;
    Some(steer_slot(nearest.min(count - 1)))
}

/// Whether `placements` hold the look-back pose.
pub fn has_back(placements: &[Placement]) -> bool {
    placements.iter().any(|p| p.slot == BACK_SLOT)
}

/// The solid of the pose `stem` at `level`, when the car has one.
fn solid_at(has: &dyn Fn(u32) -> bool, stem: &str, level: usize) -> Option<u32> {
    let hash = bstring_hash(&format!("{stem}_{}", char::from(b'A' + level.min(LEVELS - 1) as u8)));
    has(hash).then_some(hash)
}

/// The stems of the pose solids of the car in `folder`, with their slots: steering poses while they are contiguous from 0,
/// then the look back.
fn poses(folder: &str, has: &dyn Fn(u32) -> bool) -> Vec<(usize, String)> {
    let has_any = |stem: &str| (0..LEVELS).any(|level| solid_at(has, stem, level).is_some());
    // The solids are named after the folder in upper case, however the caller spelled it.
    let folder = folder.to_ascii_uppercase();
    let mut found = Vec::new();
    for k in 0..MAX_STEER_POSES {
        let stem = format!("{folder}_KIT00_DRIVER_S{k}");
        if !has_any(&stem) {
            break;
        }
        found.push((steer_slot(k), stem));
    }
    let back = format!("{folder}_KIT00_DRIVER_BACK");
    if has_any(&back) {
        found.push((BACK_SLOT, back));
    }
    found
}

/// Add the driver poses of the car in `folder` to its placements (`placements` is level `lod`, `levels` the driving levels).
/// At a level that gets any pose the plain driver placement is taken out. Returns how many poses the car has.
pub fn add(
    folder: &str,
    lod: usize,
    pool: &HashMap<u32, Arc<Solid>>,
    placements: &mut Vec<Placement>,
    levels: &mut [LodAssembly],
) -> usize {
    add_with(folder, lod, &|hash| pool.contains_key(&hash), placements, levels)
}

/// [`add`] over any way of telling whether the car has a solid.
fn add_with(
    folder: &str,
    lod: usize,
    has: &dyn Fn(u32) -> bool,
    placements: &mut Vec<Placement>,
    levels: &mut [LodAssembly],
) -> usize {
    let found = poses(folder, has);
    if found.is_empty() {
        return 0;
    }
    let place = |level: usize| -> Vec<Placement> {
        found
            .iter()
            .filter_map(|(slot, stem)| {
                solid_at(has, stem, level).map(|solid| Placement {
                    solid,
                    transform: Mat4::IDENTITY,
                    left_brake: false,
                    slot: *slot,
                    corner: None,
                })
            })
            .collect()
    };
    let put = |level: usize, list: &mut Vec<Placement>| {
        let posed = place(level);
        if posed.is_empty() {
            return;
        }
        list.retain(|p| p.slot != slot::DRIVER);
        list.extend(posed);
    };
    put(lod, placements);
    for assembly in levels.iter_mut() {
        put(assembly.level, &mut assembly.placements);
    }
    found.len()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn placement(slot: usize) -> Placement {
        Placement { solid: slot as u32, transform: Mat4::IDENTITY, left_brake: false, slot, corner: None }
    }

    #[test]
    fn the_nearest_pose_to_the_steering_is_drawn() {
        // Seven poses: 0 full left, 3 the middle, 6 full right.
        assert_eq!(slot_for(-1.0, false, 7, false), Some(steer_slot(0)));
        assert_eq!(slot_for(0.0, false, 7, false), Some(steer_slot(3)));
        assert_eq!(slot_for(1.0, false, 7, false), Some(steer_slot(6)));
        assert_eq!(slot_for(0.2, false, 7, false), Some(steer_slot(4)));
        assert_eq!(slot_for(-0.2, false, 7, false), Some(steer_slot(2)));
        assert_eq!(slot_for(0.05, false, 7, false), Some(steer_slot(3)));
        assert_eq!(slot_for(5.0, false, 7, false), Some(steer_slot(6)), "beyond full lock is full lock");
        assert_eq!(slot_for(f32::NAN, false, 7, false), Some(steer_slot(3)));
        assert_eq!(slot_for(0.7, false, 1, false), Some(steer_slot(0)), "one pose is every steering");
    }

    #[test]
    fn the_reverse_gear_looks_back_only_when_the_car_can() {
        assert_eq!(slot_for(1.0, true, 7, true), Some(BACK_SLOT));
        assert_eq!(slot_for(1.0, true, 7, false), Some(steer_slot(6)));
        assert_eq!(slot_for(0.0, true, 0, true), Some(BACK_SLOT));
    }

    #[test]
    fn a_car_without_poses_draws_none() {
        assert_eq!(slot_for(0.5, false, 0, false), None);
        assert_eq!(slot_for(0.5, true, 0, false), None);
    }

    #[test]
    fn the_pose_slots_meet_no_other_slot() {
        const { assert!(POSE_SLOT > super::super::hinged::HINGED_SLOT + 200) };
        assert!(!is_pose(slot::DRIVER) && !is_pose(POSE_SLOT - 1) && !is_pose(BACK_SLOT + 1));
        assert!(is_pose(POSE_SLOT) && is_pose(BACK_SLOT));
    }

    #[test]
    fn the_poses_are_counted_from_the_placements() {
        let placed =
            [placement(slot::BODY), placement(steer_slot(0)), placement(steer_slot(1)), placement(steer_slot(2))];
        assert_eq!(steer_count(&placed), 3);
        assert!(!has_back(&placed));
        let with_back = [placement(BACK_SLOT)];
        assert!(has_back(&with_back));
        assert_eq!(steer_count(&with_back), 0, "the look back is not a steering pose");
        assert_eq!(steer_count(&[]), 0);
    }

    fn hashes(names: &[&str]) -> HashSet<u32> {
        names.iter().map(|name| bstring_hash(name)).collect()
    }

    #[test]
    fn poses_replace_the_plain_driver_at_the_levels_that_have_them() {
        let pool = hashes(&[
            "CAR_KIT00_DRIVER_S0_A",
            "CAR_KIT00_DRIVER_S0_B",
            "CAR_KIT00_DRIVER_S1_A",
            "CAR_KIT00_DRIVER_S1_B",
            "CAR_KIT00_DRIVER_BACK_A",
            "CAR_KIT00_DRIVER_BACK_B",
            "CAR_KIT00_DRIVER_S3_A",
        ]);
        let has = |hash: u32| pool.contains(&hash);
        let plain = |level: usize| Placement {
            solid: bstring_hash(&format!("CAR_KIT00_DRIVER_{}", char::from(b'A' + level as u8))),
            ..placement(slot::DRIVER)
        };
        let mut placements = vec![placement(slot::BODY), plain(0)];
        let mut levels = vec![
            LodAssembly { level: 1, placements: vec![placement(slot::BODY), plain(1)] },
            LodAssembly { level: 2, placements: vec![placement(slot::BODY), plain(2)] },
        ];
        // S3 is not contiguous with S0, S1: it is not a pose.
        assert_eq!(add_with("CAR", 0, &has, &mut placements, &mut levels), 3);
        let slots = |list: &[Placement]| list.iter().map(|p| p.slot).collect::<Vec<_>>();
        assert_eq!(slots(&placements), [slot::BODY, steer_slot(0), steer_slot(1), BACK_SLOT]);
        assert_eq!(slots(&levels[0].placements), [slot::BODY, steer_slot(0), steer_slot(1), BACK_SLOT]);
        assert_eq!(
            slots(&levels[1].placements),
            [slot::BODY, slot::DRIVER],
            "level C has no pose: the plain driver stays"
        );
        assert_eq!(steer_count(&placements), 2);
    }

    #[test]
    fn a_car_with_no_pose_solids_is_left_alone() {
        let pool = hashes(&["CAR_KIT00_DRIVER_A"]);
        let mut placements = vec![placement(slot::DRIVER)];
        assert_eq!(add_with("CAR", 0, &|hash| pool.contains(&hash), &mut placements, &mut []), 0);
        assert_eq!(placements.len(), 1);
    }
}
