//! Cars hitting each other and the player's car, and the sounds those hits and close passes make.

use blackbox_pursuit::sight;
use glam::Vec3;

use super::TrafficWorld;
use super::traffic_events::{crash_horn, near_miss};
use crate::audio::traffic::TrafficSound;
use crate::scenes::world::drive::walls::Cast;
use crate::scenes::world::drive::{CarSim, STEP};

/// Eye height of a cop and of the player, in metres above the body's centre.
const EYE: Vec3 = Vec3::new(0.0, 0.5, 0.0);
/// A hit this close to the far end is the target itself, not an obstacle.
const SIGHT_SLACK: f32 = 0.05;

/// Horn volume for a hit this strong (N s) and above: full. Weaker hits play quieter, down to a third.
const HORN_FULL_IMPULSE: f32 = 10_000.0;
/// Near-miss whoosh volume (design value; the original's level is **[unconfirmed]**).
const NEAR_MISS_VOLUME: f32 = 0.6;

/// One body that can be hit: a car or the trailer it tows.
struct Part<'a> {
    /// Index of the car it belongs to.
    owner: usize,
    /// Its car has a hitched trailer: the car and the trailer never collide with each other.
    coupled: bool,
    sim: &'a mut CarSim,
}

/// A hit, remembered until the cars can be told: the car, the impulse (N s), where it touched (physics space), whether
/// the player did it, and how far the car is from the player (metres).
struct Hit {
    owner: usize,
    impulse: f32,
    point: Vec3,
    by_player: bool,
    from_camera: f32,
}

/// A close pass of the player past a traffic car: the car, the distance between their centres (m) and how fast the
/// two are closing (m/s, the difference of their velocities).
struct Pass {
    owner: usize,
    distance: f32,
    relative_speed: f32,
}

/// Whether the bounding spheres of two bodies touch, a cheap test before the box test.
fn near(a: &CarSim, b: &CarSim) -> bool {
    if !a.is_finite() || !b.is_finite() {
        return false;
    }
    a.body().position.distance(b.body().position) < a.collision_sweep_radius() + b.collision_sweep_radius()
}

fn horn_volume(impulse: f32) -> f32 {
    (impulse / HORN_FULL_IMPULSE).clamp(1.0 / 3.0, 1.0)
}

impl TrafficWorld {
    /// Lets the cars hit each other and the player's car (`player`). A tractor and the trailer hitched to it do
    /// not hit each other; a trailer that came loose is a body like any other. Also decides the sounds of the step:
    /// a horn for each car in a hit, a whoosh for a close pass.
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
            let (tractor, trailer) = car.sims_mut();
            parts.push(Part { owner, coupled, sim: tractor });
            if let Some(sim) = trailer {
                parts.push(Part { owner, coupled, sim });
            }
        }
        let mut hits = Vec::new();
        let mut passes = Vec::new();
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
                    });
                } else if distance < events.near_miss_distance {
                    passes.push(Pass {
                        owner: part.owner,
                        distance,
                        relative_speed: (player.body().linear_velocity - part.sim.body().linear_velocity).length(),
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
                if let Some(hit) = part.sim.collide_with(other.sim) {
                    let (near_a, near_b) = (far(&*part.sim), far(&*other.sim));
                    hits.push(Hit {
                        owner: part.owner,
                        impulse: hit.impulse,
                        point: hit.point,
                        by_player: false,
                        from_camera: near_a,
                    });
                    hits.push(Hit {
                        owner: other.owner,
                        impulse: hit.impulse,
                        point: hit.point,
                        by_player: false,
                        from_camera: near_b,
                    });
                }
            }
        }
        drop(parts);
        for hit in &hits {
            let car = &mut self.cars[hit.owner];
            if crash_horn(true, car.ready()) {
                car.sounded(events.cooldown);
                self.sounds.push(TrafficSound::Horn { index: car.horn as usize, volume: horn_volume(hit.impulse) });
            }
            if hit.by_player {
                self.hit_timer = events.hit_memory;
            }
        }
        for hit in hits {
            self.cars[hit.owner].on_hit(hit.impulse, hit.by_player, hit.point, hit.from_camera);
        }
        for pass in passes {
            let car = &mut self.cars[pass.owner];
            if near_miss(&events, pass.relative_speed, pass.distance, false, car.ready()) {
                car.sounded(events.cooldown);
                self.sounds.push(TrafficSound::NearMiss { volume: NEAR_MISS_VOLUME });
            }
        }
        self.player_speed = player.as_deref().map_or(0.0, |p| p.body().linear_velocity.length());
        self.player_position = player.as_deref().map(|p| p.body().position);
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
        let level = self.police.level().unwrap_or(1);
        let eye = player + EYE;
        self.seen_by_cops = self.cars.iter().any(|car| {
            if !(car.is_patrol() || car.is_cop() || car.is_roadblock()) {
                return false;
            }
            let cop = car.physics_position();
            let to_player = (player - cop).try_normalize().unwrap_or(Vec3::ZERO);
            let facing = car.physics_forward().dot(to_player);
            !sight::out_of_range(level, cop.distance(player), facing) && clear(cast, cop + EYE, eye)
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
