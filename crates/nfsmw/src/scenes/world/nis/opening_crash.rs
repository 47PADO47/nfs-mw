//! The opening's scene-owned traffic keeps its physical body when the animation releases it.
use glam::{Mat3, Quat, Vec3};

use super::cars::{CarSpot, Target};
use crate::scenes::world::drive::{DriveInput, FixedClock, STEP, WorldGround};
use crate::scenes::world::{WorldScene, space};

#[derive(Debug)]
struct Actor {
    index: usize,
    last: CarSpot,
    velocity: Vec3,
    yaw_rate: f32,
    released: bool,
    ground: Option<f32>,
}

#[derive(Debug, Default)]
pub(super) struct OpeningCrash {
    actors: Vec<Actor>,
    clock: FixedClock,
}

fn physical(v: Vec3) -> Vec3 {
    Vec3::from(space::to_physics(v))
}

fn orientation(q: Quat) -> Quat {
    Quat::from_mat3(&Mat3::from_cols(physical(q * Vec3::NEG_Y), physical(q * Vec3::Z), physical(q * Vec3::X)))
}

impl OpeningCrash {
    pub(super) fn mode(&self, target: Target) -> Option<&'static str> {
        let Target::Spawned(index) = target else { return None };
        let actor = self.actors.iter().find(|a| a.index == index)?;
        Some(if actor.released { "detached_physics" } else { "attached" })
    }
}

impl WorldScene {
    pub(super) fn step_opening_crash(
        &mut self,
        channels: &[(Target, CarSpot)],
        spots: &mut [(Target, CarSpot)],
        travel: &mut [[f32; 4]],
        dt: f32,
    ) {
        let camera = self.gameplay_lens(self.physics_aspect).position;
        let Some(traffic) = self.traffic.as_mut() else { return };
        let mut crash = self.nis.car_memory().map(|m| std::mem::take(&mut m.crash)).unwrap_or_default();
        let (collision, props) = self.residency.world_parts();
        let ground = WorldGround { collision, surfaces: &self.physics.surfaces };
        let mut frame = Vec::new();
        for (slot, &(target, spot)) in channels.iter().enumerate() {
            let Target::Spawned(index) = target else { continue };
            let Some((motion, _)) = self.nis.car_motion(target) else { continue };
            let Some(sim) = traffic.nis_sim(index) else { continue };
            let at = match crash.actors.iter().position(|a| a.index == index) {
                Some(at) => at,
                None => {
                    crash.actors.push(Actor {
                        index,
                        last: spot,
                        ground: None,
                        velocity: Vec3::ZERO,
                        yaw_rate: 0.0,
                        released: false,
                    });
                    crash.actors.len() - 1
                }
            };
            let actor = &mut crash.actors[at];
            // The collision rig has its own pivot, tyre clearance and animation frame.
            // Moving its box to fit the drawn wheels changes collision impulses at release.
            let p = spot.position;
            actor.ground = super::car_memory::first_ground_below(
                |from, to| space::ground_below(collision, p.x, p.y, from, to),
                p.z,
                actor.ground,
            );
            let spot =
                super::cars::lifted(spot, actor.ground, super::car_world::clearance(Some(sim.nis_ground_lift())));
            let delta = physical(spot.position - actor.last.position);
            // A cut relocates the channel, not the car's incoming momentum.
            let continuous = delta.length_squared() <= 25.0 && !actor.released;
            if motion.attached && dt > 0.0 {
                actor.velocity = if continuous { Vec3::new(delta.x, 0.0, delta.z) / dt } else { Vec3::ZERO };
                let old = physical(actor.last.rotation * Vec3::X);
                let new = physical(spot.rotation * Vec3::X);
                actor.yaw_rate =
                    if continuous { old.cross(new).y.atan2(old.x * new.x + old.z * new.z) / dt } else { 0.0 };
            }
            if !motion.attached && !actor.released {
                sim.place_scene_body(physical(actor.last.position), orientation(actor.last.rotation), actor.velocity);
                sim.body_mut().angular_velocity.y = actor.yaw_rate;
            }
            actor.released = !motion.attached;
            let start = if continuous { actor.last } else { spot };
            actor.last = spot;
            let input = DriveInput {
                throttle: if motion.burnout != 0.0 { 1.0 } else { 0.5 },
                brake: f32::from(motion.brake_lock.0 && motion.brake_lock.1),
                handbrake: motion.brake_lock.1 && !motion.brake_lock.0,
                ..Default::default()
            };
            frame.push((at, slot, start, input));
        }
        let active = frame.iter().any(|&(at, ..)| crash.actors[at].released);
        if !active {
            self.impact_fx.step(self.physics.database(), [], camera, dt);
        }
        let periods = crash.clock.advance_real(dt);
        let remainder = crash.clock.alpha() * STEP;
        for (step, &delta) in periods.iter().enumerate().filter(|_| active) {
            let lag = remainder + (periods.len() - step - 1) as f32 * STEP;
            let fraction = (1.0 - lag / dt).clamp(0.0, 1.0);
            let mut running = Vec::new();
            for &(at, _, start, input) in &frame {
                let actor = &crash.actors[at];
                let Some(sim) = traffic.nis_sim(actor.index) else { continue };
                if actor.released {
                    let state = sim.begin_step_with_delta(delta, &input, true, Some(props));
                    sim.begin_body(&state, Some(collision));
                    running.push((actor.index, state));
                } else {
                    let pos = start.position.lerp(actor.last.position, fraction);
                    let rot = start.rotation.slerp(actor.last.rotation, fraction);
                    sim.place_scene_body(physical(pos), orientation(rot), actor.velocity);
                    sim.body_mut().angular_velocity.y = actor.yaw_rate;
                }
            }
            for (n, &(at, ..)) in frame.iter().enumerate() {
                for &(other, ..) in &frame[n + 1..] {
                    let a = &crash.actors[at];
                    let b = &crash.actors[other];
                    if a.released || b.released {
                        traffic.collide_nis_cars(a.index, b.index);
                    }
                }
            }
            for (index, state) in running {
                let Some(sim) = traffic.nis_sim(index) else { continue };
                sim.finish_body(&state, &ground, Some((collision, props)));
                sim.end_step(&state, &ground, Some((collision, props)));
            }
            let mut contacts = props.take_impact_effects();
            for &(at, ..) in &frame {
                if let Some(sim) = traffic.nis_sim(crash.actors[at].index) {
                    sim.drain_impact_effects_into(&mut contacts);
                }
            }
            self.impact_fx.set_enabled(self.vehicle_effects[0]);
            self.impact_fx.step(self.physics.database(), contacts, camera, delta);
        }
        for (at, slot, ..) in frame {
            let actor = &crash.actors[at];
            if actor.released
                && let Some(sim) = traffic.nis_sim(actor.index)
            {
                let pose = sim.pose();
                travel[slot] = pose.wheels.map(|w| w.travel);
                let mut spot = CarSpot { position: pose.position, rotation: pose.rotation };
                if let Some(base) = traffic.car_pose(actor.index)
                    && let Some((treads, scale, true)) = traffic.scene_treads(actor.index, base)
                {
                    super::road_pose::released(&mut spot, &mut travel[slot], treads, scale, |p| {
                        space::ground_below(collision, p.x, p.y, p.z + 0.65, p.z - 0.5)
                    });
                }
                spots[slot].1 = spot;
            }
        }
        if let Some(memory) = self.nis.car_memory() {
            memory.crash = crash;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_crash_preserves_model_forward_and_up_in_physics() {
        let model = Quat::from_euler(glam::EulerRot::ZYX, 0.8, -0.3, 0.15);
        let body = orientation(model);
        assert!((body * Vec3::Z).abs_diff_eq(physical(model * Vec3::X), 1e-5));
        assert!((body * Vec3::Y).abs_diff_eq(physical(model * Vec3::Z), 1e-5));
    }
}
