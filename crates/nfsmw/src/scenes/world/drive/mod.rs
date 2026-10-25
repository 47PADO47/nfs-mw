//! Driving a car through the streamed city: the car, its physics on a fixed 60 Hz step, the chase
//! camera and the scripted driver. The world scene owns the streaming and decides when and where
//! the car appears; this module owns everything about the car itself.

mod access;
mod arrest;
mod breaker_camera;
mod car_detail;
mod cinematic;
mod clock;
mod extra_options;
mod grid;
mod sabotage;
pub(super) use car_detail::{DetailView, DrawOptions};
mod cracked;
mod debug;
mod effect_feed;
mod fall;
mod garage;
mod ground;
mod input;
#[cfg(test)]
mod lamp_tests;
mod look;
mod new;
mod nis_pose;
mod original_camera;
mod placement;
pub(super) use placement::Strict;
#[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
mod predict;
#[cfg(test)]
mod presentation_tests;
mod release;
mod remote;
mod render;
mod rig;
mod road_reflection;
mod script;
pub(super) mod sight;
mod sim;
mod simulation;
mod sound;
mod speedbreaker;
mod time_scale;
mod touch;
mod trace;
mod visual_tires;
pub(super) mod walls;

use crate::scenes::world::car_lights::{CarLights, LampState};
use blackbox_collision::CollisionWorld;
use glam::{Mat4, Vec3};
use nfsmw_data::car::physics::CarPhysics;

use super::effects::TireEffects;
use super::exhaust::ExhaustFlames;
use super::road::{self, Spawn};
use super::vehicle_effects::VehicleEffects;
use crate::settings::{Transmission, WheelOptions};
use crate::viewer::camera::{CameraCut, ChaseCamera, ChaseView, Followed, PursuitCamera, PursuitFeed};
pub(super) use clock::{FixedClock, STEP};
pub(super) use debug::upload_box;
pub use debug::{ContactMarkers, LEGEND as MARKER_LEGEND, MarkerMeshes};
use fall::FallWatch;
pub(super) use ground::{WorldGround, drivable_ground_below};
pub use input::DriveInput;
use input::Presses;
#[cfg(feature = "multiplayer")]
pub(super) use predict::DriveSave;
pub(super) use rig::SurfaceDamage;
pub use rig::{CarPose, CarRig};
pub use script::DriveScript;
use sim::Telemetry;
pub(super) use sim::memo::{StepMemo, SurfaceHit};
pub(super) use sim::pair_contact::PairContact;
#[cfg(feature = "multiplayer")]
pub(super) use sim::predict::SimMotion;
pub(super) use sim::{CarSim, Integration, StepStart};
use sound::SoundFeed;
pub(super) use time_scale::SpeedbreakerClock;
#[cfg(test)]
pub(super) use walls::ContactKind;
pub(super) use walls::VisualContact;

/// A scripted driver and how far into it the car is.
struct ScriptRun {
    script: DriveScript,
    time: f32,
}

/// Where the car should appear once the road around it has loaded.
#[derive(Debug, Clone, Copy)]
pub struct SpawnRequest {
    /// Map position (x, y) to look for a road near.
    pub near: [f32; 2],
    /// Heading in radians from +X to prefer, when the car should keep facing the same way.
    pub heading: Option<f32>,
    /// Put the car exactly here (a place it was known to be on the road) instead of searching.
    pub exact: Option<Spawn>,
    /// The car must go exactly to `exact` (a race's start): it waits for the ground there and is never put on
    /// another road (`placement/strict.rs`).
    pub strict: bool,
}

/// Seconds between the checks that remember the last place the car was on the road.
const GOOD_SPOT_STEPS: u32 = 30;

pub struct Drive {
    pub car_name: String,
    pub performance: nfsmw_data::car::physics::PerformancePackage,
    pub tuning: nfsmw_data::car::physics::tuning::TuningSelection,
    tunings: blackbox_vehicle::Tunings,
    pub effects: TireEffects,
    /// The brake pedal as the lamps see it, 0 to 1.
    pub brake_lamp: f32,
    /// The steering as the lamps see it, and the seconds they have blinked for.
    pub steer_lamp: f32,
    pub lamp_time: f32,
    /// The player's headlights, toggled by the Headlights action.
    pub headlights: bool,
    pub vehicle_effects: VehicleEffects,
    pub flames: ExhaustFlames,
    pub tail_trails: super::tail_light_trails::TailLightTrails,
    rig: CarRig,
    physics: CarPhysics,
    sim: Option<CarSim>,
    /// Set while the car waits for a road: the physics does not run.
    request: Option<SpawnRequest>,
    alpha: f32,
    frame_input: DriveInput,
    /// The input of a network seat, which replaces the devices' input while it is set (`remote.rs`).
    remote_input: Option<DriveInput>,
    /// Temporary racer AI controls while a world-moment camera owns the player.
    cinematic_input: Option<DriveInput>,
    /// Bust fallback holds controls until the ticket is paid.
    arrested: bool,
    /// A session's rules forbid time scaling, so the speedbreaker is off (`session_rules.rs`).
    breaker_barred: bool,
    /// The engine is sabotaged by the scripted chase: no throttle (`sabotage.rs`).
    sabotaged: bool,
    /// The car stands on a race grid before GO: the brakes are held (`grid.rs`).
    gridded: bool,
    /// A point (physics space) the car steers at by itself, with the player's steering ignored (the drag assist,
    /// `grid.rs`).
    assist_aim: Option<glam::Vec3>,
    /// The speed the next placement gives the car, m/s (a rolling start); taken by the placement.
    pending_speed: f32,
    previous: CarPose,
    current: CarPose,
    /// The pose a NIS scene gives the car while it plays (`nis_pose.rs`); `None` draws the physics pose.
    nis_pose: Option<CarPose>,
    /// The pose a multiplayer client draws its own car at: the prediction plus the smoothed correction
    /// (`predict.rs`); `None` draws the physics pose.
    net_pose: Option<CarPose>,
    telemetry: Telemetry,
    /// The speed breaker's meter (`speedbreaker.rs`).
    breaker: speedbreaker::SpeedBreaker,
    breaker_camera: breaker_camera::Zoom,
    /// What the car sound hears of the drive.
    sound: SoundFeed,
    chase: ChaseCamera,
    /// The caught pan and the arrest framing, blended over the chase view (`viewer/camera/pursuit`).
    pursuit: PursuitCamera,
    /// Console hits given before the car exists; applied when it is placed.
    queued_hits: Vec<(usize, u8, bool)>,
    /// The cracked-glass zones shown on the car (see `cracked.rs`).
    cracked: cracked::CrackedZones,
    /// The chase camera jumped to the car (it was placed on the road).
    cut: CameraCut,
    look: look::PendingLook,
    script: Option<ScriptRun>,
    /// A screenshot run drove the script in batches: once it ends the car stays where it stopped.
    batch_run: bool,
    /// Physics steps run since the car was last placed, for the once-a-second trace of a scripted run.
    steps: u32,
    /// Manual resets (the reset key) since the scene last took them; the police start their post-reset blink on them.
    manual_resets: u32,
    /// The last place the car stood on a road with its wheels down: `reset` and falling off the map
    /// bring it back here when no road is near.
    last_good: Option<Spawn>,
    /// When a strict request first failed to find its ground.
    strict_since: Option<std::time::Instant>,
    fall: FallWatch,
    /// Step count at the last road check.
    last_check: u32,
    /// Button presses that have not met a physics step yet.
    presses: Presses,
    /// Who changes gear.
    transmission: Transmission,
    /// The wheel's clutch pedal and H-shifter switches.
    wheel: WheelOptions,
    /// The contact points of `debug collisions`.
    markers: ContactMarkers,
    /// Props the car knocked over this tick, hidden by the world's next `update_events`.
    pending_knocks: Vec<u32>,
    /// How long ago the car last touched a prop (`touch.rs`).
    prop_touch: touch::PropTouch,
}

impl Drive {
    /// Who changes gear from now on (the transmission setting).
    /// The map position residency should follow, if the car is not on the road yet.
    pub fn waiting_for_road(&self) -> Option<SpawnRequest> {
        self.request
    }

    /// Heading of the car, radians from +X.
    /// The car's lamps, with the car frame and brake pedal they are drawn with (copied, so the caller can hold
    /// the effect layer meanwhile).
    /// The car's lamps, with the car frame and what lights them. Copied, so the caller can hold the effect layer.
    pub fn lamp_source(&self) -> (CarLights, super::car_light_flares::CarLightFlares, Mat4, LampState) {
        let reverse = self.sim.as_ref().is_some_and(|s| s.telemetry().gear < 0);
        let state = LampState {
            broken: self.sim.as_ref().map_or(0, |s| s.lamp_damage()),
            emergency: 0,
            brake: self.brake_lamp,
            reverse,
            steer: self.steer_lamp,
            headlights: self.headlights,
            turn_signals: false,
            time: self.lamp_time,
        };
        (self.rig.lights().clone(), self.rig.flares().clone(), self.pose().transform(), state)
    }

    /// The player car's model scale (`CarScale`).
    pub fn set_car_scale(&mut self, scale: f32) {
        self.rig.set_scale(scale);
    }

    /// The chase camera's contact shake multiplier, 0 to 1 (`[custom_hud] camera_shake`).
    pub fn set_camera_shake(&mut self, intensity: f32) {
        self.chase.set_shake_intensity(intensity);
    }

    /// Apply an impact to a zone (see `CarSim::inflict`).
    pub fn inflict(&mut self, zone: usize, severity: u8, glazed: bool) -> bool {
        match self.sim.as_mut() {
            Some(sim) => sim.inflict(zone, severity, glazed),
            None => {
                self.queued_hits.push((zone, severity, glazed));
                false
            }
        }
    }

    /// Clear the car's visible damage and punctures (the `repair` command).
    pub fn repair(&mut self) {
        if let Some(sim) = self.sim.as_mut() {
            sim.repair();
        }
    }

    /// Lift or restore the player car's rev limiter (`NoRevLimiter`).
    pub fn set_no_rev_limiter(&mut self, on: bool) {
        if let Some(sim) = self.sim.as_mut() {
            sim.set_no_rev_limiter(on);
        }
    }

    /// Turn the headlights on or off; returns the new state.
    pub fn toggle_headlights(&mut self) -> bool {
        self.headlights = !self.headlights;
        self.headlights
    }

    /// Set the headlights on or off, as the day-night clock does at its lights-on and lights-off hours.
    pub fn set_headlights(&mut self, on: bool) {
        self.headlights = on;
    }

    pub fn heading(&self) -> f32 {
        let forward = self.current.rotation * Vec3::X;
        forward.y.atan2(forward.x)
    }

    pub fn position(&self) -> Vec3 {
        self.current.position
    }

    /// Half the width and half the length of the car's body, metres.
    pub fn half_extents(&self) -> (f32, f32) {
        let half = self.physics.spec.dimension;
        (half.x, half.z)
    }

    /// Half the height of the car's body, metres: the body origin is its middle, so the feet are this far below it.
    pub fn half_height(&self) -> f32 {
        self.physics.spec.dimension.y
    }

    /// Speed along the car, m/s (negative in reverse).
    pub fn speed(&self) -> f32 {
        self.telemetry.speed_mps
    }

    /// Whether the nitrous burns now (the physics' state at the last step).
    pub fn nitrous_burning(&self) -> bool {
        self.telemetry.nos_burning
    }

    /// Whether the script (if any) has run out.
    pub fn script_finished(&self) -> bool {
        self.script.as_ref().is_none_or(|s| s.script.finished(s.time))
    }

    /// The pose to draw: between the last two physics steps.
    pub fn pose(&self) -> CarPose {
        self.previous.lerp(&self.current, self.alpha)
    }

    /// The live physical pose, including contacts not yet published to the render poses. The GPS reads it: it must
    /// follow the car as the physics has it, not the frame's blend.
    pub fn physics_pose(&self) -> CarPose {
        self.sim.as_ref().map_or(self.current, CarSim::pose)
    }

    /// Move the chase camera; `look` is the player's look input (the right stick or the mouse), which
    /// orbits the camera round the car.
    pub fn follow(&mut self, _collision: &CollisionWorld, look: (f32, f32), dt: f32) {
        if dt == 0.0 {
            // A screenshot run has no frame time: put the camera where it belongs at once.
            self.chase.snap();
        }
        self.chase.look(look.0, look.1, dt);
        let pose = self.current;
        let forward = pose.rotation * Vec3::X;
        let followed =
            Followed { position: pose.position, heading: forward.y.atan2(forward.x), speed: self.telemetry.speed_mps };
        // World presentation applies shared wall/floor/body clearance after zoom.
        self.chase.update(followed, dt, |_, _| None);
    }

    /// Hold or release the look-back control (see `ChaseCamera::set_look_back`).
    pub fn set_look_back(&mut self, on: bool) {
        self.chase.set_look_back(on);
    }

    /// Whether the camera is looking back at the car.
    pub fn looking_back(&self) -> bool {
        self.chase.looking_back()
    }

    /// Whether a caught pan or an arrest framing is running this tick (`PursuitCamera::shot_running`).
    pub fn pursuit_shot_running(&self) -> bool {
        self.pursuit.shot_running()
    }

    /// Whether a car is simulated, which is when the HUD has a state (`hud_state`).
    pub fn has_sim(&self) -> bool {
        self.sim.is_some()
    }

    /// Advances the pursuit camera's caught pan and arrest framing by `dt` of simulated time, with the chase's feed.
    pub fn update_pursuit(&mut self, dt: f32, feed: PursuitFeed) {
        self.pursuit.update(dt, feed);
    }

    pub fn camera(&self) -> ChaseView {
        self.pursuit.shot(self.chase.sample(self.alpha))
    }

    /// Cleanup and other simulation queries use the current tick, independently of rendering.
    pub fn physics_camera(&self) -> ChaseView {
        self.pursuit.shot(self.chase.view())
    }

    /// The manual resets since the last call, for the police's post-reset blink.
    pub fn take_manual_resets(&mut self) -> u32 {
        std::mem::take(&mut self.manual_resets)
    }

    pub fn take_look(&mut self, steps: u32) -> (f32, f32) {
        self.look.take(steps)
    }

    /// Turns the contact markers of `debug collisions` on or off.
    pub fn set_markers(&mut self, on: bool) {
        self.markers.set_enabled(on);
    }

    /// How many contact markers are on screen: (wall and prop contacts, tyre hits).
    pub fn marker_counts(&self) -> (usize, usize) {
        self.markers.counts()
    }

    /// Append the instances of the contact markers.
    pub fn marker_instances(&self, meshes: &MarkerMeshes, out: &mut Vec<blackbox_render::Instance>) {
        self.markers.instances(meshes, out);
    }

    pub fn hud_state(&self) -> Option<crate::hud::HudState> {
        let sim = self.sim.as_ref()?;
        let t = &self.telemetry;
        let pose = self.pose();
        let forward = pose.rotation * Vec3::X;
        Some(crate::hud::HudState {
            minimap: Some(crate::hud::MapPosition {
                position: [pose.position.x, pose.position.y],
                heading: [forward.x, forward.y],
                orientation: blackbox_minimap::Orientation::North,
            }),
            speed: t.speed_mps.abs(),
            rpm: t.rpm,
            max_rpm: t.max_rpm,
            red_line: t.red_line,
            gear: t.gear,
            shifting: t.shifting,
            shift_light: t.shift_up,
            has_nos: t.has_nos,
            nos: t.nos,
            speedbreaker: self.breaker.meter,
            speedbreaker_active: self.breaker.active(),
            has_turbo: t.has_induction,
            boost_psi: t.boost_psi,
            telemetry: sim.hud_telemetry(),
            condition: Some(sim.hud_condition()),
            ..Default::default()
        })
    }

    /// What the car sound needs of the drive; `None` while there is no car yet.
    pub fn car_sound(&mut self) -> Option<crate::audio::CarSoundState> {
        if self.sim.is_none() {
            self.sound.clear();
            return None;
        }
        Some(self.sound.state(&self.car_name, &self.telemetry, self.performance))
    }

    pub(super) fn speedbreaker_active(&self) -> bool {
        self.breaker.active()
    }
    /// The readout lines at this level: one for `Minimal`, the numbers of the HUD and the run for `Full`.
    pub fn readout(&self, level: crate::devtools::ShowReadout) -> String {
        if self.sim.is_none() || self.request.is_some() {
            return format!("{}: looking for a road...", self.car_name);
        }
        let p = self.current.position;
        if level == crate::devtools::ShowReadout::Minimal {
            return format!("{} at ({:.0}, {:.0}, {:.1})", self.car_name, p.x, p.y, p.z);
        }
        let t = &self.telemetry;
        let gear = match t.gear {
            -1 => "R".to_owned(),
            0 => "N".to_owned(),
            g => g.to_string(),
        };
        let nos = if t.nos > 0.0 { format!("  nos {:.0}%", t.nos * 100.0) } else { String::new() };
        let script = self
            .script
            .as_ref()
            .map_or(String::new(), |s| format!("  script {:.1}/{:.1} s", s.time, s.script.duration()));
        format!(
            "{:>4.0} km/h  {:>5.0} rpm  gear {gear}{nos}\n{} at ({:.0}, {:.0}, {:.1}){script}",
            t.speed_mps * 3.6,
            t.rpm,
            self.car_name,
            p.x,
            p.y,
            p.z
        )
    }
}
