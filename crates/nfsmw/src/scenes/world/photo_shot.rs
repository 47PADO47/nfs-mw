//! The camera of the win photo: the replay camera a race names (`FinishCamera`: `DefaultFinish`, `PHoldFin_17`...), an
//! ICE track whose eye and look point move in the frozen player's car space, then hold. The photo screen
//! (`InGamePhotoMaster.fng`) has a transparent window in its picture; the world seen through it, from this camera, is
//! the photo. Spec: `docs/specs/race-finish-effects.md` section 3.

use blackbox_chunk::ice::{Space, Track};
use game_install::GameDir;
use glam::{Mat4, Vec3};

use super::WorldScene;
use super::scene_space::roll;
use crate::viewer::camera::{Lens, original};
use nfsmw_data::gameplay::RaceType;
use nfsmw_data::world::finish_fx::load_replay_cameras;

/// The photo camera held on the car as it stood when the photo was taken.
pub struct PhotoShot {
    track: Track,
    /// The car's frame in render space (+x forward, +y left, +z up).
    car: Mat4,
    elapsed: f32,
}

/// The native photo screen's driving-view interval before its cinematic camera cuts in.
pub const CAMERA_DELAY: f32 = 0.75;

/// A track point in render space. Car space is the car's own frame (+x forward, +y left, +z up); hybrid space has the
/// world's axes round the car's origin; a world point is as it stands. A scene-space point has no scene here.
pub fn place(car: &Mat4, space: Space, point: [f32; 3]) -> Option<Vec3> {
    let point = Vec3::from_array(point);
    match space {
        Space::Car => Some(car.transform_point3(point)),
        Space::Hybrid => Some(car.w_axis.truncate() + point),
        Space::World => Some(point),
        Space::Scene => None,
    }
}

impl PhotoShot {
    pub fn new(track: Track, car: Mat4) -> Self {
        Self { track, car, elapsed: 0.0 }
    }

    pub fn advance(&mut self, dt: f32) -> bool {
        if !dt.is_finite() || dt <= 0.0 || self.elapsed >= CAMERA_DELAY + self.track.length {
            return false;
        }
        let before = self.current();
        self.elapsed = (self.elapsed + dt).min(CAMERA_DELAY + self.track.length);
        before != self.current()
    }

    fn fraction(&self) -> f32 {
        ((self.elapsed - CAMERA_DELAY) / self.track.length.max(0.001)).clamp(0.0, 1.0)
    }

    fn current(&self) -> Option<(Vec3, Vec3, Vec3, f32)> {
        self.camera_started().then(|| self.at(self.fraction())).flatten()
    }

    /// Whether the finish camera has cut away from the retained driving view.
    pub fn camera_started(&self) -> bool {
        self.elapsed >= CAMERA_DELAY
    }

    pub fn driving_left(&self) -> f32 {
        (CAMERA_DELAY - self.elapsed).max(0.0)
    }

    /// Average scale for this real-time slice of the host's linear finish slowdown.
    pub fn simulation_scale(&self, dt: f32) -> f32 {
        let end = (self.elapsed + dt.max(0.0)).min(CAMERA_DELAY);
        (1.0 - (self.elapsed + end) / (2.0 * CAMERA_DELAY)).clamp(0.0, 1.0)
    }

    /// The eye, the look point, the up vector and the field of view (16-bit angle) of the camera at rest, at the end of
    /// its track: the cut moves in for the first fifth of a second or so and then holds (`SprintFinish`: 3 s long, its
    /// second key, the held one, from 0.212 of it). `None` when the track has no key or a point has no place.
    #[cfg(test)]
    fn at_rest(&self) -> Option<(Vec3, Vec3, Vec3, f32)> {
        self.at(1.0)
    }

    fn at(&self, fraction: f32) -> Option<(Vec3, Vec3, Vec3, f32)> {
        let sample = self.track.sample(fraction)?;
        let eye = place(&self.car, sample.eye_space, sample.eye)?;
        let target = place(&self.car, sample.look_space, sample.look)?;
        let forward = (target - eye).normalize_or_zero();
        Some((eye, target, roll(Vec3::Z, forward, sample.roll), f32::from(sample.fov)))
    }

    /// The camera, `aspect` being width over height.
    pub fn lens(&self, aspect: f32) -> Option<Lens> {
        let (eye, target, up, fov_bangle) = self.current()?;
        Some(original::native_lens(original::Sample { eye, target, up, fov_bangle }, aspect))
    }
}

impl WorldScene {
    /// The photo camera takes the player's car where it stands now: the post-race screen is about to open over it.
    pub(super) fn start_photo_shot(&mut self, named: Option<&str>, kind: RaceType) {
        let Some(drive) = self.players.lead() else { return };
        let Some(track) = replay_track(&self.dir, named, kind == RaceType::Drag) else { return };
        log::info!("race: the win photo is framed by the replay camera {}", track.name);
        self.photo_shot = Some(PhotoShot::new(track, drive.pose().transform()));
        if let Some(drive) = self.players.lead_mut() {
            drive.set_cinematic_input(Some(Default::default()));
        }
        self.cut.arm();
    }

    /// The photo is over: the game camera is back.
    pub(super) fn end_photo_shot(&mut self) {
        if self.photo_shot.take().is_some() {
            if let Some(drive) = self.players.lead_mut() {
                drive.set_cinematic_input(None);
            }
            self.cut.arm();
        }
    }

    /// The camera of the win photo while one is up.
    pub(super) fn photo_lens(&self, aspect: f32) -> Option<Lens> {
        self.photo_shot.as_ref()?.lens(aspect)
    }

    pub(super) fn photo_driving(&self) -> bool {
        self.photo_shot.as_ref().is_some_and(|shot| !shot.camera_started())
    }

    pub(super) fn step_photo_shot(&mut self, dt: f32) -> bool {
        let Some(shot) = &mut self.photo_shot else { return false };
        let was_driving = !shot.camera_started();
        if was_driving && let Some(drive) = self.players.lead() {
            shot.car = drive.pose().transform();
        }
        let changed = shot.advance(dt);
        if was_driving && shot.camera_started() {
            self.cut.arm();
        }
        changed
    }
}

/// The track a race frames its photo with, read from the install's replay camera pack.
fn replay_track(dir: &GameDir, named: Option<&str>, drag: bool) -> Option<Track> {
    let cameras = match load_replay_cameras(dir) {
        Ok(cameras) => cameras,
        Err(error) => {
            log::warn!("the win photo has no camera: {error:#}");
            return None;
        }
    };
    cameras.finish_track(named, drag).cloned()
}

#[cfg(test)]
#[path = "photo_shot_tests.rs"]
mod tests;
