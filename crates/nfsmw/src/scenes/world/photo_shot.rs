//! The camera of the win photo: the replay camera a race names (`FinishCamera`: `DefaultFinish`, `PHoldFin_17`...), an
//! ICE track whose eye and look point are in the player's car space, settled at the end of its track. The photo screen
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
}

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
        Self { track, car }
    }

    /// The eye, the look point, the up vector and the field of view (16-bit angle) of the camera at rest, at the end of
    /// its track: the cut moves in for the first fifth of a second or so and then holds (`SprintFinish`: 3 s long, its
    /// second key, the held one, from 0.212 of it). `None` when the track has no key or a point has no place.
    fn at_rest(&self) -> Option<(Vec3, Vec3, Vec3, f32)> {
        let sample = self.track.sample(1.0)?;
        let eye = place(&self.car, sample.eye_space, sample.eye)?;
        let target = place(&self.car, sample.look_space, sample.look)?;
        let forward = (target - eye).normalize_or_zero();
        Some((eye, target, roll(Vec3::Z, forward, sample.roll), f32::from(sample.fov)))
    }

    /// The camera, `aspect` being width over height.
    pub fn lens(&self, aspect: f32) -> Option<Lens> {
        let (eye, target, up, fov_bangle) = self.at_rest()?;
        Some(original::native_lens(original::Sample { eye, target, up, fov_bangle }, aspect))
    }
}

impl WorldScene {
    /// The photo camera takes the player's car where it stands now: the post-race screen is about to open over it.
    pub(super) fn start_photo_shot(&mut self, named: Option<&str>, kind: RaceType) {
        let Some(drive) = self.drive.as_ref() else { return };
        let Some(track) = replay_track(&self.dir, named, kind == RaceType::Drag) else { return };
        log::info!("race: the win photo is framed by the replay camera {}", track.name);
        self.photo_shot = Some(PhotoShot::new(track, drive.pose().transform()));
        self.cut.arm();
    }

    /// The photo is over: the game camera is back.
    pub(super) fn end_photo_shot(&mut self) {
        if self.photo_shot.take().is_some() {
            self.cut.arm();
        }
    }

    /// The camera of the win photo while one is up.
    pub(super) fn photo_lens(&self, aspect: f32) -> Option<Lens> {
        self.photo_shot.as_ref()?.lens(aspect)
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
