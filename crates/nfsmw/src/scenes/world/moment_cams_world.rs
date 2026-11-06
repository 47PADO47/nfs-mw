//! The world's side of the game moment cameras: what starts a shot, what the shots ask of the world, and what the world
//! does while one plays. The shots are `moment_cams.rs`; spec `docs/specs/game-moment-camera.md`.

use blackbox_collision::{CollisionWorld, RayOptions};
use glam::Vec3;

use super::ai::TrafficWorld;
use super::moment_cams::{Camera, Car, Facts, MomentCams, Scene, Start, jump, rules};
use super::{WorldScene, space};
use crate::settings::game_moments::MomentKind;
use crate::viewer::camera::Lens;

/// What the world keeps for the shots between steps.
#[derive(Default)]
pub(super) struct MomentShots {
    pub cams: MomentCams,
    /// The player's car had a wheel down at the last step (the takeoff is the step it has none).
    grounded: bool,
    pursued: bool,
    /// Seconds a started pursuit has waited for a cop to look at (counted while the screen is not faded) **[ours]**.
    pursuit_wait: Option<f32>,
}

/// How long a started pursuit waits for its first cop before the pursuit shot is dropped (s) **[ours]**: the original
/// places the first cop as the pursuit starts, the rewrite a moment later (after a jump, once its model is in).
const PURSUIT_WAIT: f32 = 3.0;

/// The world's answers to the shots' questions, render space.
struct World<'a> {
    collision: &'a CollisionWorld,
    traffic: Option<&'a TrafficWorld>,
}

/// The original's line test raises both ends half a metre and meets the world's faces and walls.
const SIGHT_LIFT: f32 = 0.5;

impl Scene for World<'_> {
    fn clear(&self, a: Vec3, b: Vec3) -> bool {
        let lift = |p: Vec3| space::to_physics(p + Vec3::Z * SIGHT_LIFT);
        let options = RayOptions { faces: true, barriers: true, exclude: 0, skip_groups: true };
        self.collision.ray_cast(lift(a), lift(b), &options).is_none()
    }

    fn ground(&self, at: Vec3) -> Option<f32> {
        space::ground_below(self.collision, at.x, at.y, at.z + 4.0, at.z - 60.0)
    }

    fn road_ahead(&self, at: Vec3, forward: Vec3, ahead: f32) -> Option<(Vec3, [Vec3; 2])> {
        self.traffic?.road_ahead(at, forward, ahead)
    }
}

impl jump::Ground for World<'_> {
    fn under(&self, at: Vec3) -> Option<(f32, f32)> {
        let [x, _, z] = space::to_physics(at);
        let hit = super::drive::drivable_ground_below(self.collision, x, z, at.z + 4.0, at.z - 60.0)?;
        Some((hit.point[1], hit.normal[1]))
    }

    fn clear(&self, a: Vec3, b: Vec3) -> bool {
        Scene::clear(self, a, b)
    }
}

impl WorldScene {
    /// The console line about the game moment cameras.
    pub(super) fn moment_cams_status(&self) -> String {
        self.moment_shots.cams.status()
    }

    /// One frame of `real_dt` seconds: starts the shots the rules allow and plays the current one.
    pub(super) fn step_moment_cams(&mut self, real_dt: f32) {
        let scripted = self.moment.hides_hud() || self.nis.is_playing() || self.story_movie.is_playing();
        let Some(drive) = self.players.lead().filter(|_| !scripted) else {
            if self.moment_shots.cams.playing() {
                self.moment_shots.cams.stop();
            }
            return;
        };
        let pose = drive.pose();
        let view = drive.player_view();
        let car = Car {
            position: pose.position,
            velocity: view.velocity,
            forward: pose.rotation * Vec3::X,
            left: pose.rotation * Vec3::Y,
            up: pose.rotation * Vec3::Z,
        };
        let grounded = drive.sim().is_some_and(|sim| sim.telemetry().wheels_on_ground > 0);
        let takeoff = std::mem::replace(&mut self.moment_shots.grounded, grounded) && !grounded;
        let speedbreaker = drive.speedbreaker_camera_on();
        let flight = drive.sim().map(|sim| sim.flight_start(view.speed));
        let pursued = self.traffic.as_ref().is_some_and(TrafficWorld::is_pursuing);
        let started = !std::mem::replace(&mut self.moment_shots.pursued, pursued) && pursued;
        let world = World { collision: self.residency.collision(), traffic: self.traffic.as_ref() };
        // No shot starts during the speedbreaker or its fade (the original's gate for every moment).
        let open = |kind: MomentKind| !speedbreaker && self.moment_allowed(kind);
        let (open_pursuit, open_jumps, open_crashes) =
            (open(MomentKind::Pursuit), open(MomentKind::Jumps), open(MomentKind::Crashes));
        let mut starts = Vec::new();
        if started && open_pursuit && self.race_allows_pursuit_shot() {
            self.moment_shots.pursuit_wait = Some(0.0);
        }
        if !pursued {
            self.moment_shots.pursuit_wait = None;
        }
        let faded = self.screen_fade_up();
        if let Some(wait) = self.moment_shots.pursuit_wait.as_mut().filter(|_| !faded) {
            *wait += real_dt;
        }
        let waiting = self.moment_shots.pursuit_wait.is_some_and(|wait| wait <= PURSUIT_WAIT);
        if self.moment_shots.pursuit_wait.is_some() && !waiting {
            self.moment_shots.pursuit_wait = None;
        }
        if takeoff && open_jumps && !self.moment_shots.cams.car_shot() {
            let zone = self.camera_environment.regions.first_of_kind_at(JUMP_ZONE, car.position.truncate()).is_some();
            let flight = flight.and_then(|(start, gravity)| {
                let start = space::to_render(start.to_array());
                jump::predict(start, car.velocity, gravity, &world)
            });
            let speed = car.velocity.length();
            if let Some(flight) = flight.filter(|&flight| rules::jump_worth_a_shot(flight, speed, zone)) {
                starts.push(Start::Jump(rules::jump_secs(flight.airtime)));
            }
        }
        if open_crashes && self.crash_coming(&car, grounded) {
            starts.push(Start::Crash { roadblock: true });
        }
        // The camera and the cops in its sight matter only to a shot that starts now or is about to look for a cop.
        let looking = !starts.is_empty() || waiting || self.moment_shots.cams.wants_camera();
        let lens = match looking {
            true => self.moment_camera_now(car.position),
            false => Camera { eye: car.position, target: car.position, fov: 0.0 },
        };
        let cops: Vec<Vec3> = match looking {
            true => self.traffic.as_ref().map(TrafficWorld::cops_seeing_player).unwrap_or_default(),
            false => Vec::new(),
        };
        let cops: Vec<Vec3> = cops.into_iter().filter(|&cop| Scene::clear(&world, lens.eye, cop)).collect();
        // The pursuit shot starts once a cop it can look at is there, and not under a fade.
        if waiting && !faded && !cops.is_empty() && open_pursuit {
            self.moment_shots.pursuit_wait = None;
            starts.insert(0, Start::Pursuit);
        }
        let facts = Facts { car, pursued, camera: lens, cops: &cops, scene: &world };
        let cams = &mut self.moment_shots.cams;
        for start in starts {
            cams.start(start, &facts);
        }
        cams.update(real_dt, &facts);
        if cams.take_cut() {
            self.cut.arm();
        }
    }

    /// Whether the player's car is about to crash through a roadblock: the pursuit's nearest roadblock car is between
    /// 3 m and a quarter second ahead, and some car stands on the line of travel within that reach.
    fn crash_coming(&self, car: &Car, grounded: bool) -> bool {
        let Some(traffic) = self.traffic.as_ref().filter(|traffic| traffic.is_pursuing()) else { return false };
        let speed = car.velocity.length();
        let near = traffic.nearest_roadblock_car(car.position).is_some_and(|d| d > 3.0 && d < 0.25 * speed);
        near && rules::crash_ahead(car.position, car.forward, speed, grounded, &traffic.live_car_positions())
    }

    /// In a race the pursuit shot plays only after the race's first second, and never in a pursuit race.
    fn race_allows_pursuit_shot(&self) -> bool {
        let Some(race) = self.race.as_ref() else { return true };
        let elapsed = race.engine.as_ref().map_or(0.0, |engine| engine.elapsed());
        !race.origin.race.params.pursuit_race && elapsed >= 1.0
    }

    /// The camera as the rules see it now, for the shots that start from it: its eye, the point on its line of sight
    /// level with the player's car, and its lens in 16-bit angle units.
    fn moment_camera_now(&self, car: Vec3) -> Camera {
        let lens = self.simulation_lens(self.physics_aspect);
        let reach = (car - lens.position).dot(lens.forward).max(1.0);
        Camera { eye: lens.position, target: lens.position + lens.forward * reach, fov: fov_bangle(lens.fov_y_degrees) }
    }

    /// The lens of the shot that plays, if one does.
    pub(super) fn moment_cams_lens(&self, aspect: f32) -> Option<Lens> {
        let drive = self.players.lead()?;
        let pose = drive.pose();
        let car = Car {
            position: pose.position,
            velocity: drive.player_view().velocity,
            forward: pose.rotation * Vec3::X,
            left: pose.rotation * Vec3::Y,
            up: pose.rotation * Vec3::Z,
        };
        let (eye, target, fov) = self.moment_shots.cams.view(&car)?;
        let sample = crate::viewer::camera::original::Sample { eye, target, up: Vec3::Z, fov_bangle: fov };
        Some(crate::viewer::camera::original::native_lens(sample, aspect))
    }

    /// The shots' moment sounds, at the player's car.
    pub(super) fn take_moment_cam_sounds(&mut self) -> Vec<crate::audio::WorldEffectAudio> {
        let sounds = self.moment_shots.cams.take_sounds();
        let Some(drive) = self.players.lead() else { return Vec::new() };
        let position = drive.pose().position;
        sounds
            .into_iter()
            .map(|key| crate::audio::WorldEffectAudio {
                effect: 0,
                zone: nfsmw_data::sound::CollisionZone::Default,
                position,
                magnitude: 1.0,
                moment: Some(blackbox_attrib::vlt_hash(key)),
            })
            .collect()
    }
}

/// The track zone kind of the jump camera's lower thresholds (`JumpCam`).
const JUMP_ZONE: u32 = 12;

/// The 16-bit lens angle the native optics turn into this vertical field of view (`viewer/camera/original/projection.rs`).
fn fov_bangle(fov_y_degrees: f32) -> f32 {
    let half = (fov_y_degrees.to_radians() * 0.5).tan();
    let vertical = (1.215 * half).atan() / std::f32::consts::TAU * 65536.0;
    vertical / (1.15 * 0.43434)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lens_angle_survives_the_native_optics() {
        for bangle in [3000.0, 8000.0, 12000.0] {
            let sample = crate::viewer::camera::original::Sample {
                eye: Vec3::ZERO,
                target: Vec3::X,
                up: Vec3::Z,
                fov_bangle: bangle,
            };
            let lens = crate::viewer::camera::original::native_lens(sample, 16.0 / 9.0);
            assert!((fov_bangle(lens.fov_y_degrees) - bangle).abs() < 30.0, "{bangle}");
        }
    }
}
