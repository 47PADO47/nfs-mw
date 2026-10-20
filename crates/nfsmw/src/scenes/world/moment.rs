//! The cinematic moment of a pursuit breaker: a short camera cut of the collapse while the world goes on. Spec:
//! `docs/specs/pursuit-breaker-moment.md`.
//!
//! The breaker's event sequence asks for a scene ("WMGasStation"). The original plays it only when the player's car
//! touched the structure a moment ago and a cop chasing the player is close (`Moment::request`). It then finds the
//! scene's marker nearest the player, puts the scene's origin there and plays the scene's camera track for the
//! track's length. The scene holds no animation of its own: the structure falls by the world's own physics.

use std::sync::Arc;

use blackbox_chunk::ice::{CameraSample, Space, Track};
use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::world::moment::{Marker, MomentData, scene_hash};

use crate::viewer::camera::{Lens, original};

/// The scene's marker must score under this (metres): its distance plus the turn penalty.
pub const MARKER_RANGE: f32 = 75.0;
/// The player's car must have touched the structure this recently (seconds).
pub const TOUCH_WINDOW: f32 = 2.0;
/// A chasing cop must be this close behind the player's car, and up to [`REACH_AHEAD`] when it is right ahead.
const REACH_BEHIND: f32 = 80.0;
const REACH_AHEAD: f32 = 20.0;
/// One quarter turn in 16-bit angle units: the scene faces its marker's angle turned this far.
const QUARTER_TURN: u16 = 16384;

/// Why a moment did not play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Another moment is playing (the scene system takes one at a time).
    Playing,
    /// The player is being arrested.
    Arrested,
    /// The player's car did not touch the structure within [`TOUCH_WINDOW`].
    NotTouched,
    /// No cop chasing the player is within reach.
    NoPursuer,
    /// The install has no data for the scene.
    NoScene,
    /// No marker of the scene's kind scores under [`MARKER_RANGE`].
    NoMarker,
    /// The scene has no camera track.
    NoCamera,
}

/// What the world knows when a breaker asks for a moment.
pub struct Facts<'a> {
    /// Seconds since the player's car last touched a prop, if it ever did.
    pub touch_age: Option<f32>,
    /// The player's car, render space.
    pub player: Vec3,
    pub forward: Vec3,
    /// Every cop chasing the player, render space.
    pub pursuers: &'a [Vec3],
    pub arrested: bool,
}

/// Whether a cop chasing the player is close enough: the reach is 20 m for a cop right ahead and 80 m for one right
/// behind, by the cosine between the car's heading and the way from the cop to the car. Distances are horizontal.
pub fn pursuer_in_reach(player: Vec3, forward: Vec3, pursuers: &[Vec3]) -> bool {
    let facing = Vec3::new(forward.x, forward.y, 0.0).normalize_or_zero();
    pursuers.iter().any(|&cop| {
        let way = Vec3::new(player.x - cop.x, player.y - cop.y, 0.0);
        let cosine = way.normalize_or_zero().dot(facing);
        way.length() < REACH_AHEAD + (REACH_BEHIND - REACH_AHEAD) * 0.5 * (1.0 + cosine)
    })
}

/// A 16-bit angle (65536 per turn) of a direction on the render ground plane, counted from +x towards +y.
pub fn heading_angle(forward: Vec3) -> u16 {
    let turns = forward.y.atan2(forward.x) / std::f32::consts::TAU;
    ((turns.rem_euclid(1.0) * 65536.0) as u32 & 0xFFFF) as u16
}

/// The scene's origin and heading: where the markers' scene is stood on the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Anchor {
    origin: Vec3,
    /// The marker's angle less a quarter turn, as 16-bit units.
    turn: u16,
}

impl Anchor {
    /// A scene-space point in render space: turned about the up axis, then moved to the origin.
    fn to_render(self, point: [f32; 3]) -> Vec3 {
        let (sin, cos) = (f32::from(self.turn) / 65536.0 * std::f32::consts::TAU).sin_cos();
        let (x, y) = (point[0], point[1]);
        self.origin + Vec3::new(x * cos - y * sin, x * sin + y * cos, point[2])
    }
}

struct Playing {
    scene: String,
    track: Track,
    anchor: Anchor,
    elapsed: f32,
}

impl Playing {
    fn fraction(&self) -> f32 {
        (self.elapsed / self.track.length.max(1e-3)).clamp(0.0, 1.0)
    }

    fn sample(&self) -> Option<CameraSample> {
        self.track.sample(self.fraction())
    }
}

/// The moment being played, if any, and the data it needs.
#[derive(Default)]
pub struct Moment {
    data: Option<Arc<MomentData>>,
    data_failed: bool,
    play: Option<Playing>,
    /// The last request and what became of it, for the console.
    last: Option<String>,
    /// The cut ended or began since the camera last asked.
    cut: bool,
}

impl Moment {
    /// Reads the install's moment data once. Without it every request is refused.
    pub fn prepare(&mut self, dir: &GameDir) {
        if self.data.is_some() || self.data_failed {
            return;
        }
        match nfsmw_data::world::moment::load(dir) {
            Ok(data) => self.data = Some(data),
            Err(error) => {
                log::warn!("cinematic moments are off: {error:#}");
                self.data_failed = true;
            }
        }
    }

    /// A breaker asks for the moment of `scene`. `ground` gives the ground height under a point (render space).
    pub fn request(
        &mut self,
        scene: &str,
        facts: &Facts<'_>,
        ground: impl Fn(Vec3) -> Option<f32>,
    ) -> Result<(), Refusal> {
        let result = self.try_request(scene, facts, ground);
        self.last = Some(match result {
            Ok(()) => format!("{scene}: played"),
            Err(refusal) => format!("{scene}: not played ({refusal:?})"),
        });
        log::info!("pursuit breaker moment: {}", self.last.as_deref().unwrap_or_default());
        result
    }

    fn try_request(
        &mut self,
        scene: &str,
        facts: &Facts<'_>,
        ground: impl Fn(Vec3) -> Option<f32>,
    ) -> Result<(), Refusal> {
        if self.play.is_some() {
            return Err(Refusal::Playing);
        }
        if facts.arrested {
            return Err(Refusal::Arrested);
        }
        if !facts.touch_age.is_some_and(|age| age <= TOUCH_WINDOW) {
            return Err(Refusal::NotTouched);
        }
        if !pursuer_in_reach(facts.player, facts.forward, facts.pursuers) {
            return Err(Refusal::NoPursuer);
        }
        let data = self.data.clone().ok_or(Refusal::NoScene)?;
        let hash = scene_hash(scene);
        let (marker, score) =
            data.closest_marker(hash, facts.player, heading_angle(facts.forward)).ok_or(Refusal::NoScene)?;
        if score >= MARKER_RANGE {
            return Err(Refusal::NoMarker);
        }
        let track = data.track(hash, 0).cloned().ok_or(Refusal::NoCamera)?;
        let anchor = anchor_of(marker, ground);
        self.play = Some(Playing { scene: scene.to_owned(), track, anchor, elapsed: 0.0 });
        self.cut = true;
        Ok(())
    }

    /// Plays `dt` seconds of simulated time. The cut ends when its track has run out.
    pub fn advance(&mut self, dt: f32) {
        let Some(play) = self.play.as_mut() else { return };
        play.elapsed += dt.max(0.0);
        if play.elapsed >= play.track.length {
            self.play = None;
            self.cut = true;
        }
    }

    pub fn is_playing(&self) -> bool {
        self.play.is_some()
    }

    /// The HUD is hidden for the whole cut.
    pub fn hides_hud(&self) -> bool {
        self.play.is_some()
    }

    /// The busted meter does not move during a cut.
    pub fn holds_busted(&self) -> bool {
        self.play.is_some()
    }

    /// The share of real time the world runs at (the track's simulation speed, 1 when no cut plays).
    pub fn sim_speed(&self) -> f32 {
        self.play.as_ref().and_then(Playing::sample).map_or(1.0, |s| s.sim_speed)
    }

    /// Whether the camera jumped since this was last called: the cut began or ended.
    pub fn take_cut(&mut self) -> bool {
        std::mem::take(&mut self.cut)
    }

    /// The camera of the cut, `aspect` being width over height.
    pub fn lens(&self, aspect: f32) -> Option<Lens> {
        let play = self.play.as_ref()?;
        let sample = play.sample()?;
        let eye = place(&play.anchor, sample.eye_space, sample.eye)?;
        let target = place(&play.anchor, sample.look_space, sample.look)?;
        let forward = (target - eye).normalize_or_zero();
        let up = roll(Vec3::Z, forward, sample.roll);
        Some(original::native_lens(original::Sample { eye, target, up, fov_bangle: f32::from(sample.fov) }, aspect))
    }

    /// One line for the console.
    pub fn status(&self) -> String {
        let now = match &self.play {
            Some(play) => format!(
                "playing {} {:.1} of {:.1} s at {:.0}% speed",
                play.scene,
                play.elapsed,
                play.track.length,
                self.sim_speed() * 100.0
            ),
            None => "none playing".to_owned(),
        };
        format!("cinematic moment: {now}; last: {}", self.last.as_deref().unwrap_or("none"))
    }
}

/// The marker's scene anchor: its position stood on the ground under it, its angle less a quarter turn.
fn anchor_of(marker: Marker, ground: impl Fn(Vec3) -> Option<f32>) -> Anchor {
    let z = ground(marker.position).unwrap_or(marker.position.z);
    Anchor { origin: Vec3::new(marker.position.x, marker.position.y, z), turn: marker.angle.wrapping_sub(QUARTER_TURN) }
}

/// A track point in render space. Only the scene's own space is placed: car, world and hybrid spaces belong to
/// cameras that follow a car, and no moment uses them.
fn place(anchor: &Anchor, space: Space, point: [f32; 3]) -> Option<Vec3> {
    match space {
        Space::Scene => Some(anchor.to_render(point)),
        Space::World => Some(Vec3::from_array(point)),
        Space::Car | Space::Hybrid => None,
    }
}

/// `up` turned about `forward` by `roll` (16-bit units).
fn roll(up: Vec3, forward: Vec3, roll: u16) -> Vec3 {
    if roll == 0 || forward == Vec3::ZERO {
        return up;
    }
    glam::Quat::from_axis_angle(forward, f32::from(roll) / 65536.0 * std::f32::consts::TAU) * up
}

#[cfg(test)]
#[path = "moment_tests.rs"]
mod tests;
