//! Vehicle contacts and ordered moving traffic audio snapshots.

use blackbox_pursuit::sight;
use glam::Vec3;

use super::TrafficWorld;

use crate::audio::traffic::{TrafficFrame, TrafficSound};
use crate::scenes::world::drive::walls::Cast;
use crate::scenes::world::drive::{CarSim, STEP};

/// Eye height of a cop and of the player, in metres above the body's centre.
const EYE: Vec3 = Vec3::new(0.0, 0.5, 0.0);
/// A hit this close to the far end is the target itself, not an obstacle.
const SIGHT_SLACK: f32 = 0.05;

/// One body that can be hit: a car or the trailer it tows.
struct Part<'a> {
    /// Index of the car it belongs to.
    owner: usize,
    /// Its car has a hitched trailer: the car and the trailer never collide with each other.
    coupled: bool,
    /// The class of its car's driver, as the crash gate reads it.
    class: DriverClass,
    sim: &'a mut CarSim,
}

/// The driver of a car, as the crash gate reads it (`ai-traffic-obstacles.md` section 5.1): the player is `Human`
/// (class 0), a car cruising the streets is `Traffic` (class 1), and a cop is `Cop` (class 2): a chasing cop, a patrol
/// car or a roadblock car. The original's racer (class 3) has no car in this port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DriverClass {
    Human,
    Traffic,
    Cop,
}

/// A hit, remembered until the cars can be told: the car, the impulse (N s), where it touched (physics space), whether
/// the player did it, how far the car is from the player (metres), and the class and speed (m/s) of the party that hit
/// it.
pub(super) struct Hit {
    owner: usize,
    impulse: f32,
    point: Vec3,
    by_player: bool,
    from_camera: f32,
    striker: DriverClass,
    speed: f32,
}

/// A close pass of the player past a traffic car: the car, the distance between their centres (m) and how fast the
/// two are closing (m/s, the difference of their velocities).
pub(super) struct Pass {
    owner: usize,
    distance: f32,
    relative_speed: f32,
}

/// What the player struck since the police last read it: a cop for the first time (an assault), or a prop (a breakable).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Strikes {
    pub cop: bool,
    pub property: bool,
}

/// Whether the bounding spheres of two bodies touch, a cheap test before the box test.
fn near(a: &CarSim, b: &CarSim) -> bool {
    if !a.is_finite() || !b.is_finite() {
        return false;
    }
    a.body().position.distance(b.body().position) < a.collision_sweep_radius() + b.collision_sweep_radius()
}

fn planar_relative_speed(a: Vec3, b: Vec3) -> f32 {
    let velocity = a - b;
    velocity.x.hypot(velocity.z)
}

impl TrafficWorld {
    /// Lets the cars hit each other and the player's car (`player`). A tractor and the trailer hitched to it do
    /// not hit each other; a trailer that came loose is a body like any other. Also decides the sounds of the step:
    /// an authored whoosh for a close pass. Autonomous horns belong to the audio producer.
    pub fn collide(&mut self, mut player: Option<&mut CarSim>) {
        self.hit_timer = (self.hit_timer - STEP).max(0.0);
        for car in &mut self.cars {
            car.tick_quiet();
        }
        let events = self.events;
        let focus = player.as_deref().map(|p| p.body().position);
        let far = |sim: &CarSim| focus.map_or(f32::INFINITY, |f| f.distance(sim.body().position));
        let mut parts: Vec<Part<'_>> = Vec::new();
        for (owner, car) in self.cars.iter_mut().enumerate() {
            let coupled = car.is_coupled();
            let class = car.driver_class();
            let (tractor, trailer) = car.sims_mut();
            parts.push(Part { owner, coupled, class, sim: tractor });
            if let Some(sim) = trailer {
                parts.push(Part { owner, coupled, class, sim });
            }
        }
        let mut hits = std::mem::take(&mut self.hits);
        hits.clear();
        let mut passes = std::mem::take(&mut self.passes);
        passes.clear();
        for i in 0..parts.len() {
            let (head, tail) = parts.split_at_mut(i + 1);
            let part = &mut head[i];
            // A body whose state already went bad must not push the player's car into a bad state too.
            if !part.sim.is_finite() {
                continue;
            }
            if let Some(player) = player.as_deref_mut() {
                // A pass is judged on the centres, which can be further apart than the bounding spheres touch.
                let distance = player.body().position.distance(part.sim.body().position);
                let player_speed = player.body().linear_velocity.length();
                if near(player, part.sim)
                    && let Some(hit) = player.collide_with(part.sim)
                {
                    let from_camera = far(&*part.sim);
                    hits.push(Hit {
                        owner: part.owner,
                        impulse: hit.impulse,
                        point: hit.point,
                        by_player: true,
                        from_camera,
                        striker: DriverClass::Human,
                        speed: player_speed,
                    });
                } else if distance < events.near_miss_distance {
                    passes.push(Pass {
                        owner: part.owner,
                        distance,
                        relative_speed: planar_relative_speed(
                            player.body().linear_velocity,
                            part.sim.body().linear_velocity,
                        ),
                    });
                }
            }
            for other in tail {
                if part.coupled && part.owner == other.owner {
                    continue;
                }
                if !near(part.sim, other.sim) {
                    continue;
                }
                let speed_a = part.sim.body().linear_velocity.length();
                let speed_b = other.sim.body().linear_velocity.length();
                if let Some(hit) = part.sim.collide_with(other.sim) {
                    let (near_a, near_b) = (far(&*part.sim), far(&*other.sim));
                    hits.push(Hit {
                        owner: part.owner,
                        impulse: hit.impulse,
                        point: hit.point,
                        by_player: false,
                        from_camera: near_a,
                        striker: other.class,
                        speed: speed_b,
                    });
                    hits.push(Hit {
                        owner: other.owner,
                        impulse: hit.impulse,
                        point: hit.point,
                        by_player: false,
                        from_camera: near_b,
                        striker: part.class,
                        speed: speed_a,
                    });
                }
            }
        }
        drop(parts);
        for hit in &hits {
            if hit.by_player {
                self.hit_timer = events.hit_memory;
            }
        }
        for hit in hits.drain(..) {
            let first_damage = self.cars[hit.owner].on_hit(
                hit.impulse,
                hit.by_player,
                hit.striker,
                hit.speed,
                hit.point,
                hit.from_camera,
            );
            self.strikes.cop |= first_damage;
        }
        self.hits = hits;
        for car in &mut self.cars {
            let distance = focus.map_or(f32::INFINITY, |f| f.distance(car.physics_position()));
            let Some(duration) = car.take_honk(distance) else { continue };
            self.sounds.push(TrafficSound::Horn { emitter: car.audio_emitter().id, duration });
        }
        if let Some(player) = player.as_deref_mut() {
            self.strikes.property |= player.take_prop_struck();
            self.spike_player(player);
        }
        for pass in passes.drain(..) {
            let car = &mut self.cars[pass.owner];
            if pass.relative_speed > 10.0 && pass.distance < 10.0 && car.ready() {
                car.sounded(events.cooldown);
                self.sounds.push(TrafficSound::NearMiss {
                    emitter: car.audio_emitter().id,
                    relative_speed: pass.relative_speed,
                });
            }
        }
        self.passes = passes;
        if let Some(player) = player.as_deref() {
            let body = player.body();
            use crate::scenes::world::space;
            self.sounds.push(TrafficSound::Frame(TrafficFrame {
                dt: STEP,
                listener_position: space::to_render(body.position.to_array()),
                listener_forward: space::to_render((body.orientation * Vec3::Z).to_array()),
                listener_velocity: space::to_render(body.linear_velocity.to_array()),
                emitters: self.cars.iter().map(|car| car.audio_emitter()).collect(),
            }));
        }
        self.player_speed = player.as_deref().map_or(0.0, |p| p.body().linear_velocity.length());
        self.player_position = player.as_deref().map(|p| p.body().position);
    }

    /// What the player struck since the last call: a cop for the first time, or a prop. The police read it once a frame.
    pub fn take_strikes(&mut self) -> Strikes {
        std::mem::take(&mut self.strikes)
    }

    /// Whether a cop sees the player, tested every `TEST_PERIOD` seconds of simulated time (`CanSeeTarget`, ai-pursuit
    /// section 4.1). Every cop car counts: patrol cars, the chase's cops and the roadblock's. A cop sees the player when
    /// the heat's range and rear cone allow it and the world's faces do not block the line from its eyes to the
    /// player's. Between tests the last answer holds.
    pub fn update_sight(&mut self, dt: f32, cast: Cast<'_>) {
        self.sight_clock += dt;
        if self.sight_clock < sight::TEST_PERIOD {
            return;
        }
        self.sight_clock = self.sight_clock.rem_euclid(sight::TEST_PERIOD);
        let Some(player) = self.player_position else {
            self.seen_by_cops = false;
            return;
        };
        let level = self.police.patrol_heat();
        let hidden = self.hidden_from_cars(player);
        let eye = player + EYE;
        self.seen_by_cops = self.cars.iter_mut().fold(false, |seen, car| {
            if !car.is_live_police() {
                return seen;
            }
            let cop = car.physics_position();
            let to_player = (player - cop).try_normalize().unwrap_or(Vec3::ZERO);
            let facing = car.physics_forward().dot(to_player);
            let distance = cop.distance(player);
            let out_of_range = match car.is_cop() || car.is_roadblock() {
                true => sight::out_of_range(level, distance, facing),
                false => sight::patrol_out_of_range(distance, facing),
            };
            let visible = !out_of_range && clear(cast, cop + EYE, eye);
            seen | car.report_sight(hidden, visible)
        });
    }
}

/// Whether the segment from `from` to `to` is free: no solid hit before its end (physics space).
fn clear(cast: Cast<'_>, from: Vec3, to: Vec3) -> bool {
    match cast(from, to) {
        None => true,
        Some(hit) => hit.point.distance(from) >= from.distance(to) - SIGHT_SLACK,
    }
}

#[cfg(test)]
mod sight_tests {
    use super::*;
    use crate::scenes::world::drive::walls::Hit;

    fn hit_at(point: Vec3) -> Hit {
        Hit { point, normal: Vec3::Y, prop: None, info: None }
    }

    #[test]
    fn an_open_line_is_clear() {
        let open = |_: Vec3, _: Vec3| None;
        assert!(clear(&open, Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0)));
    }

    #[test]
    fn a_wall_between_the_eyes_blocks_the_sight() {
        let wall = |_: Vec3, _: Vec3| Some(hit_at(Vec3::new(40.0, 0.5, 0.0)));
        assert!(!clear(&wall, Vec3::new(0.0, 0.5, 0.0), Vec3::new(100.0, 0.5, 0.0)));
    }

    #[test]
    fn a_hit_at_the_target_does_not_block_it() {
        let target = Vec3::new(100.0, 0.5, 0.0);
        let at_target = |_: Vec3, _: Vec3| Some(hit_at(target));
        assert!(clear(&at_target, Vec3::new(0.0, 0.5, 0.0), target));
    }
}

#[cfg(test)]
mod tests;
