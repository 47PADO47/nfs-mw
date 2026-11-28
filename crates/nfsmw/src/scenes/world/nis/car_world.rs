//! The cars of a NIS scene in the world: which real car each car channel drives, and the poses the scene gives them each
//! frame. The channels and their poses are in `cars.rs`, and which kind of car each channel takes in `cop_cars.rs`. The
//! player's override is `Drive::set_nis_pose` and the traffic's is `TrafficWorld::set_nis_pose`. Rules:
//! `docs/specs/nis-player.md` sections 7.2 to 7.5.

use glam::{Quat, Vec3};

use super::car_memory::{CarMemory, first_ground_below};
use super::cars::{self, CarSpot, Target};
use super::cop_cars::{self, CopSource, SlotKind};
use super::cop_wait::WAIT_LIMIT;
use crate::scenes::world::ai::ModelState;
use crate::scenes::world::drive::CarPose;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::{WorldScene, space};

impl WorldScene {
    /// Links the playing scene's car channels once. Race starts swap the human into the primary channel without changing
    /// physical slots; other scenes put the player on channel 1. A civilian
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
            let found = match (self.nis.race_start_scene(), kind) {
                (true, Some(SlotKind::Player | SlotKind::Civilian)) => self.race.as_ref().and_then(|race| {
                    let grid: Vec<usize> = race.plan.iter().map(|plan| plan.grid_slot).collect();
                    let id = race_start_racer(&grid, slot)?;
                    if id == 0 {
                        return self.players.lead().map(|_| Target::Player);
                    }
                    self.traffic.as_ref()?.racer_car_index(id).map(Target::Traffic)
                }),
                (_, Some(SlotKind::Player)) => self.players.lead().map(|_| Target::Player),
                (_, Some(SlotKind::Civilian)) => self
                    .traffic
                    .as_ref()
                    .and_then(|traffic| traffic.nearest_car(spot.position, &taken))
                    .map(Target::Traffic),
                (_, Some(SlotKind::Cop)) => self.link_cop(slot, spot, &taken),
                (_, None) => None,
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
            let Some(target) = self.link_cop(slot, spot, &taken) else { continue };
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

    /// The car a cop channel `slot` at `spot` takes: the vehicle of the scene's special car list, made for the scene
    /// once its model has loaded (`None` while it loads); without a list, or when its model failed, the nearest free
    /// police car of the chase, or a police car the scene spawns there; an arrest tries the spawn first (`cop_cars.rs`).
    fn link_cop(&mut self, slot: usize, spot: CarSpot, taken: &[usize]) -> Option<Target> {
        let listed = self.nis_listed_vehicle(slot);
        // A list that makes no car for this channel: the original's channel has none.
        if listed == Some(None) {
            return None;
        }
        let (collision, _) = self.residency.world_parts();
        let surfaces = &self.physics.surfaces;
        if let (Some(Some(vehicle)), Some(traffic)) = (listed, self.traffic.as_mut()) {
            traffic.request_nis_vehicle(vehicle);
            match traffic.nis_vehicle_state(vehicle) {
                ModelState::Loaded => {
                    let at = traffic.spawn_nis_vehicle(
                        vehicle,
                        spot.position,
                        heading_of(spot.rotation),
                        collision,
                        surfaces,
                    );
                    return at.map(Target::Spawned);
                }
                ModelState::Pending => return None,
                ModelState::Failed => {
                    log::info!("NIS: no {vehicle} for cop channel {}: a police car stands in", slot - 7)
                }
            }
        }
        let source = self.nis.playing_kind().map_or(CopSource::BorrowFirst, cop_cars::cop_source);
        let traffic = self.traffic.as_mut()?;
        let borrowed = traffic.nearest_police(spot.position, taken);
        cop_cars::choose_cop(source, borrowed, || {
            traffic.spawn_nis_cop(spot.position, heading_of(spot.rotation), collision, surfaces)
        })
    }

    /// Drives the playing scene's cars this frame: each linked car takes its spot, lifted onto the ground under it (the
    /// ground it last stood on when none is found this frame).
    pub(in crate::scenes::world) fn drive_nis_cars(&mut self) {
        let dt = match (self.nis.scene_time(), self.nis.car_memory()) {
            (Some(time), Some(memory)) => memory.tick(time),
            _ => 0.0,
        };
        let mut spots = self.nis.car_spots();
        self.stage_extra_intro_cars(&mut spots);
        let opening = self.nis.playing_name().is_some_and(|name| name.eq_ignore_ascii_case("IntroNisDD"));
        let crash_channels = opening.then(|| spots.clone());
        let mut travels = Vec::with_capacity(spots.len());
        for (target, channel) in &mut spots {
            let target = *target;
            let spot = *channel;
            // A detached car coasts on from where its channel left it (`coast.rs`).
            let (attached, braking) = self
                .nis
                .car_motion(target)
                .map_or((true, false), |(motion, _)| (motion.attached, motion.brake_lock.0 || motion.brake_lock.1));
            let spot = match self.nis.car_memory() {
                _ if opening && matches!(target, Target::Spawned(_)) => spot,
                Some(memory) => memory.coasts.place(target, spot, attached, braking, dt),
                None => spot,
            };
            let last = self.nis.car_memory().and_then(|memory| memory.ground(target, spot.position));
            let ground = self.ground_at(spot.position);
            if let (Some(height), Some(memory)) = (ground, self.nis.car_memory()) {
                memory.set_ground(target, spot.position.with_z(height));
            }
            let ground = ground.or(last);
            if let Some(road) = ground
                && let Some((treads, scale, _)) = self.target_treads(target)
                && let Some((placed, travel)) = super::road_pose::place(spot, treads, scale, road, |p, reach| {
                    space::ground_below(self.residency.collision(), p.x, p.y, road + reach, road - reach)
                })
            {
                *channel = placed;
                travels.push(travel);
                continue;
            }
            travels.push([0.0; 4]);
            let lift = match target {
                Target::Player => self.players.lead().and_then(|drive| drive.sim()).map(|sim| sim.nis_ground_lift()),
                Target::Traffic(index) | Target::Spawned(index) => {
                    self.traffic.as_ref().and_then(|traffic| traffic.nis_ground_lift(index))
                }
            };
            *channel = cars::lifted(spot, ground, clearance(lift));
        }
        if let Some(channels) = crash_channels {
            self.step_opening_crash(&channels, &mut spots, &mut travels, dt);
        }
        for ((target, spot), travel) in spots.into_iter().zip(travels) {
            self.set_nis_car(target, Some(spot), dt, travel);
        }
        // The race simulation is frozen during a scene. Its world emitters still run on scene time.
        if dt > 0.0 {
            let (_, props) = self.residency.world_parts();
            self.object_fx.request(props.take_particle_commands());
            self.object_fx.step(props, dt);
            self.ambient_fx.step(dt);
            self.finish_flares.step(dt);
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
    /// (`car_memory.rs`), or clears its override with `None`, `dt` scene seconds after the last frame.
    fn set_nis_car(&mut self, target: Target, spot: Option<CarSpot>, dt: f32, travel: [f32; 4]) {
        let motion = self.nis.car_motion(target);
        self.apply_nis_car_state(target, motion.map(|(motion, _)| motion));
        let held = motion.map_or([false; 4], |(motion, _)| locked_wheels(motion));
        let mut memory = self.nis.car_memory();
        match target {
            Target::Player => {
                let Some(drive) = self.players.lead_mut() else { return };
                let base = drive.pose();
                let radii = drive.sim().map_or([DEFAULT_RADIUS; 4], |sim| sim.wheel_radii());
                let pose = spot.map(|spot| {
                    let mut spin = rolled_wheels(memory.as_deref_mut(), target, spot, radii, base, held);
                    if let (Some((motion, time)), Some(sim)) = (motion, drive.sim()) {
                        let (wheelbase, width, driven) = sim.nis_geometry();
                        let tilt = motion.rotation(time, wheelbase, width);
                        if let Some(memory) = memory {
                            spin = memory.burnout(target, motion.burnout * dt, radii, driven).unwrap_or(spin);
                        }
                        drive.set_nis_body_tilt(tilt);
                    }
                    posed(spot, base, spin, travel)
                });
                match pose {
                    Some(pose) => drive.move_nis_pose(pose, dt),
                    None => drive.set_nis_pose(None),
                }
            }
            Target::Traffic(index) | Target::Spawned(index) => {
                let Some(traffic) = self.traffic.as_mut() else { return };
                let radii = traffic.car_wheel_radii(index).unwrap_or([DEFAULT_RADIUS; 4]);
                let pose = spot.and_then(|spot| {
                    let base = traffic.car_pose(index)?;
                    let mut spin = rolled_wheels(memory.as_deref_mut(), target, spot, radii, base, held);
                    if let (Some((motion, time)), Some((wheelbase, width, driven))) =
                        (motion, traffic.nis_geometry(index))
                    {
                        if let Some(memory) = memory {
                            spin = memory.burnout(target, motion.burnout * dt, radii, driven).unwrap_or(spin);
                        }
                        traffic.set_nis_body_tilt(index, motion.rotation(time, wheelbase, width));
                    }
                    Some(posed(spot, base, spin, travel))
                });
                traffic.set_nis_pose(index, pose);
            }
        }
    }

    /// Places the player's car at `spot`, lifted onto the ground under it, at rest. The drive does it the way a race
    /// places its cars, once the world around it has loaded.
    fn stand_player(&mut self, spot: CarSpot) {
        let spot = cars::lifted(spot, self.ground_at(spot.position), 0.0);
        let Some(drive) = self.players.lead_mut() else { return };
        drive.place_exactly(Spawn { position: spot.position, heading: heading_of(spot.rotation) }, 0.0);
    }

    /// A missed clip-height cast may start under the street. Retry at the mapped road deck.
    fn ground_at(&self, at: Vec3) -> Option<f32> {
        let probe = |from, to| space::ground_below(self.residency.collision(), at.x, at.y, from, to);
        first_ground_below(probe, at.z, None).or_else(|| {
            let (net, index) = self.traffic.as_ref()?.road_graph();
            let point = Vec3::from(space::to_physics(at));
            let road = blackbox_roads::closest_segment(net, index, point, Vec3::ZERO, 0.0, Default::default())?;
            super::road_pose::mapped_ground(at, space::to_render(road.point.to_array()), probe)
        })
    }
}

/// The racer (its index in the race's plan, 0 being the player) that the car channel `slot` of a race's start scene
/// drives, for the plan's grid slots `grid` (the player's first). The channel of the player's grid slot is the primary
/// one (`actor_grid_slot`). `None` when no racer stands on the channel's grid slot: the channel stays off.
pub(super) fn race_start_racer(grid: &[usize], slot: usize) -> Option<usize> {
    let grid_slot = actor_grid_slot(*grid.first()?, slot);
    grid.iter().position(|&planned| planned == grid_slot)
}

/// The primary cinematic channel follows the human, independently of the physical starting order.
pub(super) fn actor_grid_slot(player: usize, channel: usize) -> usize {
    match channel {
        0 => player,
        slot if slot == player => 0,
        slot => slot,
    }
}

/// A wheel radius for a car whose physics is not there, metres.
const DEFAULT_RADIUS: f32 = 0.33;
/// The heights of a model's origin above its tyre contacts that are taken as they are, metres [ours]. A car whose wheels
/// have not found the ground (a racer frozen on a grid whose world has not streamed in) reports contacts metres away,
/// which stood the scene's car 12 m in the air in the `IntroNisDD` capture; such a car takes [`DEFAULT_CLEARANCE`].
const CLEARANCE_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.5;
const DEFAULT_CLEARANCE: f32 = 0.35;

pub(super) fn clearance(lift: Option<f32>) -> f32 {
    lift.filter(|h| CLEARANCE_RANGE.contains(h)).unwrap_or(DEFAULT_CLEARANCE)
}

/// Combine scene motion with the suspension travel at each displayed wheel.
fn posed(spot: CarSpot, base: CarPose, spin: [f32; 4], travel: [f32; 4]) -> CarPose {
    let mut wheels = base.wheels;
    for (i, wheel) in wheels.iter_mut().enumerate() {
        wheel.spin = spin[i];
        wheel.travel = travel[i];
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
    held: [bool; 4],
) -> [f32; 4] {
    let start = base.wheels.map(|w| w.spin);
    let Some(memory) = memory else { return start };
    memory.roll(target, spot.position, spot.rotation * Vec3::X, radii, start, held)
}

/// The wheels a scene's brake lock holds still, in the model's wheel order (front left, front right, rear left, rear
/// right).
fn locked_wheels(motion: super::car_events::CarMotion) -> [bool; 4] {
    let (front, rear) = (motion.locked(false), motion.locked(true));
    [front, front, rear, rear]
}

/// The heading of a car's rotation, in radians from +X as [`Spawn::heading`] counts it. The car's forward axis is its
/// +X in render space.
pub(super) fn heading_of(rotation: Quat) -> f32 {
    let forward = rotation * Vec3::X;
    forward.y.atan2(forward.x)
}
