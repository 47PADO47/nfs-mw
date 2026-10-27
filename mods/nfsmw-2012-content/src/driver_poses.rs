//! The 2012 driver's poses: the seated figure baked at a few steering inputs and at a look back, as solids the game picks
//! between while the car is driven (`docs/modding/car-customization.md`, "The driver's poses").
//!
//! The 2012 game animates the driver: the character is skinned to a 46-bone skeleton, a state machine scrubs a one-second clip
//! with the steering input and reaches the arms to the grips on the wheel (`docs/formats/chameleon-driver-animation.md`). The
//! 2005 engine draws a driver as a static solid, so this mod poses the figure itself, once, when it converts the car: the
//! clip at [`STEER_POSES`] evenly spaced inputs from full left to full right with both hands reached to the car's wheel grips,
//! and the look-back clip for the reverse gear. Each pose is a full copy of the figure (the finest level) named
//! `<FOLDER>_KIT00_DRIVER_S<k>` (k = 0 full left ... [`STEER_POSES`] - 1 full right) and `<FOLDER>_KIT00_DRIVER_BACK`, at the
//! levels `A` and `B`; the game draws the one the steering input is nearest to. The plain `<FOLDER>_KIT00_DRIVER_A..D` stays:
//! it is the driver at the levels that have no pose, and the one a game without this feature draws.

use chameleon_bundle::Bundle;
use chameleon_vehicle::{DriverPose, Lod, driver_grips, driver_setup};

use crate::source::Source;

/// How many steering poses are baked: odd, so the middle one is the wheel straight. The game reads however many the car has.
pub const STEER_POSES: usize = 7;
/// The levels of detail of the figure that are posed (the finest only; the levels `A` and `B` of the car are both it).
pub const POSED_LEVELS: usize = 1;
/// The letters of the car's levels the poses have solids for.
pub const POSED_LETTERS: usize = 2;
/// The suffix of the look-back pose.
pub const BACK: &str = "BACK";

/// One baked pose: the suffix of its solids' names and the figure in it, seated, in the 2012 engine's space.
#[derive(Debug, Clone)]
pub struct BakedPose {
    pub suffix: String,
    pub lods: Vec<Lod>,
}

/// The steering input pose `k` is made for: -1 for 0, 1 for the last.
pub fn steer_of(k: usize) -> f32 {
    -1.0 + 2.0 * k as f32 / (STEER_POSES - 1) as f32
}

/// The suffix of steering pose `k`.
pub fn steer_suffix(k: usize) -> String {
    format!("S{k}")
}

/// The poses of the driver of the car whose bundle is `bytes`: the steering poses in order, then the look back. Empty when the
/// car has no driver, or the install's figure has no rig that reads.
pub fn bake(source: &Source, bytes: &[u8]) -> Vec<BakedPose> {
    let Ok(bundle) = Bundle::parse(bytes) else { return Vec::new() };
    let Some(setup) = driver_setup(&bundle) else { return Vec::new() };
    let Some(seat) = setup.seat else { return Vec::new() };
    let Some(figure) = source.driver_figure(setup.character) else { return Vec::new() };
    let Some(rig) = &figure.rig else { return Vec::new() };
    let grips = driver_grips(&bundle, &setup, seat);
    let mut baked = Vec::with_capacity(STEER_POSES + 1);
    for k in 0..STEER_POSES {
        let pose = rig.pose(DriverPose::Steering(steer_of(k)), grips);
        baked.push(BakedPose { suffix: steer_suffix(k), lods: rig.posed(&figure.driver, &pose, seat, POSED_LEVELS) });
    }
    let back = rig.pose(DriverPose::LookBack, None);
    baked.push(BakedPose { suffix: BACK.to_owned(), lods: rig.posed(&figure.driver, &back, seat, POSED_LEVELS) });
    baked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steering_poses_run_from_full_left_to_full_right_through_the_middle() {
        assert_eq!(steer_of(0), -1.0);
        assert_eq!(steer_of(STEER_POSES - 1), 1.0);
        assert!(steer_of(STEER_POSES / 2).abs() < 1e-6, "the middle pose is the wheel straight");
        assert!((0..STEER_POSES - 1).all(|k| steer_of(k) < steer_of(k + 1)));
    }

    #[test]
    fn the_poses_are_named_for_the_game_to_find() {
        assert_eq!(steer_suffix(0), "S0");
        assert_eq!(steer_suffix(6), "S6");
        assert_eq!(BACK, "BACK");
        const { assert!(STEER_POSES % 2 == 1) };
    }

    /// With the install: every car with a driver gets the steering poses and the look back, each with a finest level that
    /// differs from the seated figure, and the middle pose stays close to it.
    #[test]
    #[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
    fn the_poses_of_real_cars_differ_from_the_seated_figure_and_from_each_other() {
        let Some(root) = std::env::var_os("MW2012_GAME_DIR") else { return };
        let source = Source::open(std::path::Path::new(&root)).unwrap();
        let file = source.cars().iter().find(|car| car.number == 1085007).expect("the Porsche 911 Carrera S");
        let bytes = source.bundle_bytes(file).unwrap();
        let poses = bake(&source, &bytes);
        assert_eq!(poses.len(), STEER_POSES + 1);
        assert_eq!(poses.last().unwrap().suffix, BACK);
        let finest = |pose: &BakedPose| -> Vec<[f32; 3]> {
            pose.lods[0].iter().flat_map(|mesh| mesh.vertices.iter().map(|v| v.position)).collect()
        };
        let (left, middle, right, back) = (
            finest(&poses[0]),
            finest(&poses[STEER_POSES / 2]),
            finest(&poses[STEER_POSES - 1]),
            finest(&poses[STEER_POSES]),
        );
        let apart = |a: &[[f32; 3]], b: &[[f32; 3]]| {
            a.iter().zip(b).map(|(p, q)| (0..3).map(|k| (p[k] - q[k]).powi(2)).sum::<f32>().sqrt()).fold(0.0, f32::max)
        };
        assert_eq!(left.len(), middle.len());
        assert!(apart(&left, &middle) > 0.02 && apart(&right, &middle) > 0.02, "full lock moves the body");
        assert!(
            apart(&left, &right) > apart(&left, &middle),
            "full left is further from full right than from the middle"
        );
        assert!(apart(&back, &middle) > 0.1, "the look back turns the head and shoulders");
    }
}
