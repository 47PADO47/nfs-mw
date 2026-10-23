//! The cars of a NIS scene in the world: which real car each car channel drives, and the poses the scene gives them each
//! frame. The channels and their poses are in `cars.rs`, and which kind of car each channel takes in `cop_cars.rs`. The
//! player's override is `Drive::set_nis_pose` and the traffic's is `TrafficWorld::set_nis_pose`. Rules:
//! `docs/specs/nis-player.md` sections 7.2 to 7.5.

use glam::{Quat, Vec3};

use super::car_memory::{CarMemory, first_ground_below};
use super::cars::{self, CarSpot, Target};
use super::cop_cars::{self, SlotKind};
use super::cop_wait::WAIT_LIMIT;
use crate::scenes::world::drive::CarPose;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::{WorldScene, space};

impl WorldScene {
    /// Links the playing scene's car channels to the real cars, once per scene. Channel 1 is the player's car. A civilian
    /// channel (`car2`..`car8`) takes the nearest traffic car that no channel has taken yet, never a police car. A cop
    /// channel (`cop1`..`cop8`) takes the nearest free police car, or a police car the scene spawns when none is free; when
    /// it can take neither yet, it waits for the police car model (`retry_nis_cops`). A channel with no car to take stays
    /// off. The log says which cop channels borrowed a car, which spawned one, which wait, and which channels got none.
    pub(in crate::scenes::world) fn link_nis_cars(&mut self) {
        if !self.nis.needs_links() {
            return;
        }
        let mut links = Vec::new();
        let mut taken = Vec::new();
        let mut borrowed = Vec::new();
        let mut spawned = Vec::new();
        let mut waiting = Vec::new();
        let mut missing = Vec::new();
        for (slot, spot) in self.nis.car_slots() {
            let kind = cop_cars::slot_kind(slot);
            let found = match kind {
                Some(SlotKind::Player) => self.players.lead().map(|_| Target::Player),
                Some(SlotKind::Civilian) => self
                    .traffic
                    .as_ref()
                    .and_then(|traffic| traffic.nearest_car(spot.position, &taken))
                    .map(Target::Traffic),
                Some(SlotKind::Cop) => self.link_cop(spot, &taken),
                None => None,
            };
            let Some(target) = found else {
                match (kind, self.traffic.is_some()) {
                    (Some(SlotKind::Cop), true) => waiting.push(slot),
                    _ => missing.push(slot + 1),
                }
                continue;
            };
            if let Target::Traffic(index) | Target::Spawned(index) = target {
                taken.push(index);
            }
            match (kind, target) {
                (Some(SlotKind::Cop), Target::Traffic(_)) => borrowed.push(slot + 1),
                (Some(SlotKind::Cop), Target::Spawned(_)) => spawned.push(slot + 1),
                _ => {}
            }
            links.push((slot, target));
        }
        let name = self.nis.playing_name().unwrap_or_default();
        let waiting_channels: Vec<usize> = waiting.iter().map(|slot| slot + 1).collect();
        log::info!(
            "NIS {name}: cop channel(s) {borrowed:?} borrow a police car, {spawned:?} spawn one, {waiting_channels:?} wait for one; no car for channel(s) {missing:?}"
        );
        let asks = !waiting.is_empty();
        self.nis.set_links(links);
        self.nis.wait_for_cops(waiting);
        if !asks {
            return;
        }
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.request_nis_cop();
        }
    }

    /// Retries the playing scene's waiting cop channels, once a frame, with the frame's real time `dt`: a channel takes the
    /// nearest free police car or a car the scene spawns, as [`WorldScene::link_nis_cars`] does. A channel still waiting
    /// when [`WAIT_LIMIT`] real seconds have passed gives up and stays off; the log says so once.
    pub(in crate::scenes::world) fn retry_nis_cops(&mut self, dt: f32) {
        for slot in self.nis.cop_waiting().to_vec() {
            let spot = self.nis.car_slots().into_iter().find(|&(s, _)| s == slot).map(|(_, spot)| spot);
            let Some(spot) = spot else { continue };
            let taken = self.nis.linked_cars();
            let Some(target) = self.link_cop(spot, &taken) else { continue };
            log::info!(
                "NIS {}: cop channel {} gets its police car",
                self.nis.playing_name().unwrap_or_default(),
                slot + 1
            );
            self.nis.link_waiting_cop(slot, target);
        }
        let given_up = self.nis.tick_cop_wait(dt);
        if given_up.is_empty() {
            return;
        }
        let channels: Vec<usize> = given_up.iter().map(|slot| slot + 1).collect();
        let name = self.nis.playing_name().unwrap_or_default();
        log::info!("NIS {name}: no police car for cop channel(s) {channels:?} within {WAIT_LIMIT} s; they stay off");
    }

    /// The police car a cop channel at `spot` takes: the nearest free one of the chase, or a car the scene spawns there.
    fn link_cop(&mut self, spot: CarSpot, taken: &[usize]) -> Option<Target> {
        let (collision, _) = self.residency.world_parts();
        let surfaces = &self.physics.surfaces;
        let traffic = self.traffic.as_mut()?;
        let borrowed = traffic.nearest_police(spot.position, taken);
        cop_cars::choose_cop(borrowed, || {
            traffic.spawn_nis_cop(spot.position, heading_of(spot.rotation), collision, surfaces)
        })
    }

    /// Drives the playing scene's cars this frame: each linked car takes its spot, lifted onto the ground under it (the
    /// ground it last stood on when none is found this frame).
    pub(in crate::scenes::world) fn drive_nis_cars(&mut self) {
        for (target, spot) in self.nis.car_spots() {
            let last = self.nis.car_memory().and_then(|memory| memory.ground(target));
            let ground = self.ground_at(spot.position, last);
            if let (Some(height), Some(memory)) = (ground, self.nis.car_memory()) {
                memory.set_ground(target, height);
            }
            let spot = cars::lifted(spot, ground);
            self.set_nis_car(target, Some(spot));
        }
    }

    /// Ends the scene's car work. The player's car stands where the scene left it, at rest. Every car draws its physics
    /// pose again: a borrowed police car goes back to the chase. The police cars the scene spawned are removed.
    pub(in crate::scenes::world) fn finish_nis_cars(&mut self) {
        let mut spawned = Vec::new();
        for (target, spot) in self.nis.take_final_cars().unwrap_or_default() {
            match target {
                Target::Player => self.stand_player(spot),
                Target::Spawned(index) => spawned.push(index),
                Target::Traffic(_) => {}
            }
        }
        if let Some(drive) = self.players.lead_mut() {
            drive.set_nis_pose(None);
        }
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.clear_nis_poses();
            traffic.remove_cars(&spawned);
        }
    }

    /// Gives the car of `target` the pose `spot`, its wheels rolled with the distance the scene moved it
    /// (`car_memory.rs`), or clears its override with `None`.
    fn set_nis_car(&mut self, target: Target, spot: Option<CarSpot>) {
        let memory = self.nis.car_memory();
        match target {
            Target::Player => {
                let Some(drive) = self.players.lead_mut() else { return };
                let base = drive.pose();
                let radii = drive.sim().map_or([DEFAULT_RADIUS; 4], |sim| sim.wheel_radii());
                let pose = spot.map(|spot| posed(spot, base, rolled_wheels(memory, target, spot, radii, base)));
                drive.set_nis_pose(pose);
            }
            Target::Traffic(index) | Target::Spawned(index) => {
                let Some(traffic) = self.traffic.as_mut() else { return };
                let radii = traffic.car_wheel_radii(index).unwrap_or([DEFAULT_RADIUS; 4]);
                let pose = spot.and_then(|spot| {
                    let base = traffic.car_pose(index)?;
                    Some(posed(spot, base, rolled_wheels(memory, target, spot, radii, base)))
                });
                traffic.set_nis_pose(index, pose);
            }
        }
    }

    /// Places the player's car at `spot`, lifted onto the ground under it, at rest. The drive does it the way a race
    /// places its cars, once the world around it has loaded.
    fn stand_player(&mut self, spot: CarSpot) {
        let spot = cars::lifted(spot, self.ground_at(spot.position, None));
        let Some(drive) = self.players.lead_mut() else { return };
        drive.place_exactly(Spawn { position: spot.position, heading: heading_of(spot.rotation) }, 0.0);
    }

    /// The ground height under a car at `at` (render space), from a little above it down (`car_memory.rs`), else `last`.
    fn ground_at(&mut self, at: Vec3, last: Option<f32>) -> Option<f32> {
        let (collision, _) = self.residency.world_parts();
        first_ground_below(|from, to| space::ground_below(collision, at.x, at.y, from, to), at.z, last)
    }
}

/// A wheel radius for a car whose physics is not there, metres.
const DEFAULT_RADIUS: f32 = 0.33;

/// A car's pose from its spot and its base pose: the spot's position and rotation, the base's steering and suspension,
/// and the wheels turned to `spin`.
fn posed(spot: CarSpot, base: CarPose, spin: [f32; 4]) -> CarPose {
    let mut wheels = base.wheels;
    for (wheel, spin) in wheels.iter_mut().zip(spin) {
        wheel.spin = spin;
    }
    CarPose { position: spot.position, rotation: spot.rotation, wheels }
}

/// How far the wheels of `target` at `spot` have turned this frame, from the scene's memory (the base pose's wheels
/// when no scene memory is there).
fn rolled_wheels(
    memory: Option<&mut CarMemory>,
    target: Target,
    spot: CarSpot,
    radii: [f32; 4],
    base: CarPose,
) -> [f32; 4] {
    let start = base.wheels.map(|w| w.spin);
    let Some(memory) = memory else { return start };
    memory.roll(target, spot.position, spot.rotation * Vec3::X, radii, start)
}

/// The heading of a car's rotation, in radians from +X as [`Spawn::heading`] counts it. The car's forward axis is its
/// +X in render space.
fn heading_of(rotation: Quat) -> f32 {
    let forward = rotation * Vec3::X;
    forward.y.atan2(forward.x)
}
