//! The cars of a NIS scene in the world: which real car each car channel drives, and the poses the scene gives them each
//! frame. The channels and their poses are in `cars.rs`. The player's override is `Drive::set_nis_pose` and the traffic's
//! is `TrafficWorld::set_nis_pose`. Rules: `docs/specs/nis-player.md` sections 7.2 to 7.5.

use glam::{Quat, Vec3};

use super::cars::{self, CarSpot, Target};
use crate::scenes::world::drive::CarPose;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::{WorldScene, space};

/// How far above and below a car the ground is looked for, in metres.
const GROUND_REACH: f32 = 20.0;

impl WorldScene {
    /// Links the playing scene's car channels to the real cars, once per scene. Channel 1 is the player's car. Every other
    /// channel takes the nearest traffic car that no channel has taken yet. A channel with no car to take stays off, and
    /// the log says which.
    pub(in crate::scenes::world) fn link_nis_cars(&mut self) {
        if !self.nis.needs_links() {
            return;
        }
        let mut links = Vec::new();
        let mut taken = Vec::new();
        let mut missing = Vec::new();
        for (slot, at) in self.nis.car_slots() {
            let found = match slot {
                0 => self.drive.as_ref().map(|_| Target::Player),
                _ => self.traffic.as_ref().and_then(|traffic| traffic.nearest_car(at, &taken)).map(Target::Traffic),
            };
            let Some(target) = found else {
                missing.push(slot + 1);
                continue;
            };
            if let Target::Traffic(index) = target {
                taken.push(index);
            }
            links.push((slot, target));
        }
        if !missing.is_empty() {
            let name = self.nis.playing_name().unwrap_or_default();
            log::info!("NIS {name}: no car for channel(s) {missing:?}; they stay off");
        }
        self.nis.set_links(links);
    }

    /// Drives the playing scene's cars this frame: each linked car takes its spot, lifted onto the ground under it.
    pub(in crate::scenes::world) fn drive_nis_cars(&mut self) {
        for (target, spot) in self.nis.car_spots() {
            let spot = cars::lifted(spot, self.ground_at(spot.position));
            self.set_nis_car(target, Some(spot));
        }
    }

    /// Ends the scene's car work. The player's car stands where the scene left it, at rest, and every car draws its
    /// physics pose again.
    pub(in crate::scenes::world) fn finish_nis_cars(&mut self) {
        for (target, spot) in self.nis.take_final_cars().unwrap_or_default() {
            match target {
                Target::Player => self.stand_player(spot),
                Target::Traffic(_) => {}
            }
        }
        if let Some(drive) = self.drive.as_mut() {
            drive.set_nis_pose(None);
        }
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.clear_nis_poses();
        }
    }

    /// Gives the car of `target` the pose `spot`, or clears its override with `None`.
    fn set_nis_car(&mut self, target: Target, spot: Option<CarSpot>) {
        match target {
            Target::Player => {
                let Some(drive) = self.drive.as_mut() else { return };
                let pose = spot.map(|spot| posed(spot, drive.pose()));
                drive.set_nis_pose(pose);
            }
            Target::Traffic(index) => {
                let Some(traffic) = self.traffic.as_mut() else { return };
                let pose = spot.and_then(|spot| traffic.car_pose(index).map(|base| posed(spot, base)));
                traffic.set_nis_pose(index, pose);
            }
        }
    }

    /// Places the player's car at `spot`, lifted onto the ground under it, at rest. The drive does it the way a race
    /// places its cars, once the world around it has loaded.
    fn stand_player(&mut self, spot: CarSpot) {
        let spot = cars::lifted(spot, self.ground_at(spot.position));
        let Some(drive) = self.drive.as_mut() else { return };
        drive.place_exactly(Spawn { position: spot.position, heading: heading_of(spot.rotation) }, 0.0);
    }

    /// The ground height under `at` (render space), looked for within [`GROUND_REACH`] above and below it.
    fn ground_at(&mut self, at: Vec3) -> Option<f32> {
        let (collision, _) = self.residency.world_parts();
        space::ground_below(collision, at.x, at.y, at.z + GROUND_REACH, at.z - GROUND_REACH)
    }
}

/// A car's pose from its spot and its base pose: the spot's position and rotation, with the base's wheels.
fn posed(spot: CarSpot, base: CarPose) -> CarPose {
    CarPose { position: spot.position, rotation: spot.rotation, ..base }
}

/// The heading of a car's rotation, in radians from +X as [`Spawn::heading`] counts it. The car's forward axis is its
/// +X in render space.
fn heading_of(rotation: Quat) -> f32 {
    let forward = rotation * Vec3::X;
    forward.y.atan2(forward.x)
}
