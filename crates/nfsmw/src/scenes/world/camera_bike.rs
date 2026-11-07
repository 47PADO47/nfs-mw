//! The cameras mounted on a bike: the one ahead of it that looks back at the bike and the road, and the one inside the rider's
//! helmet (`viewer/camera/bike.rs`, `docs/specs/ride6-bike-cameras.md`). They take the places of the original game's bumper and
//! hood views, which sit on a car's bumper and bonnet, when the player's car is a bike that has a camera set.

use glam::Vec3;

use super::drive::{Drive, RiderVisibility};
use super::{View, WorldScene};
use crate::settings::CameraMode;
use crate::viewer::camera::{Lens, bike};

/// The share of the camera on a thrown rider (`Drive::rider_follow`) from which his helmet and visor are drawn again: below it the
/// camera is nearly at his eyes (coming back, or just leaving) and they stay out of the picture.
const HELMET_SHOWN_FROM_FOLLOW: f32 = 0.15;

/// Which of a bike's mounted cameras a camera setting asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Shot {
    /// Ahead of the bike, looking back at it and the road (the bumper view's place).
    Front,
    /// Inside the rider's helmet, at his eyes (the hood view's place).
    Helmet,
}

impl Shot {
    pub(super) fn of(mode: CameraMode) -> Option<Self> {
        match mode {
            CameraMode::Bumper => Some(Self::Front),
            CameraMode::Hood => Some(Self::Helmet),
            _ => None,
        }
    }
}

impl WorldScene {
    /// The mounted camera in use: the lead drives a bike with a camera set, in the driving view, and the setting is one of
    /// the cameras a bike mounts.
    pub(super) fn bike_shot(&self) -> Option<Shot> {
        let drive = self.players.lead().filter(|_| self.view == View::Chase)?;
        drive.bike_cameras()?;
        Shot::of(self.camera_mode)
    }

    /// Whether the camera is inside the rider's helmet (so his helmet and visor are not drawn): not while the camera is on a thrown
    /// rider (`rider_follow`, a chase camera that shows him whole) but from the moment it is back at the eyes.
    pub(super) fn bike_helmet_active(&self) -> bool {
        let followed = self.players.lead().is_some_and(|drive| drive.rider_follow() >= HELMET_SHOWN_FROM_FOLLOW);
        self.bike_shot() == Some(Shot::Helmet) && !followed
    }

    /// What the camera shows of the lead's rider: all of him outside his helmet, all but the helmet and visor from inside it, and
    /// none of him when the helmet camera looks back over the tail (the camera is then inside his own back).
    pub(super) fn bike_rider_visibility(&self) -> RiderVisibility {
        let looking_back = self.players.lead().is_some_and(|drive| drive.camera_looking_back(self.camera_mode));
        RiderVisibility::of(self.bike_helmet_active(), looking_back)
    }

    /// The lens of the mounted camera on `drive`'s bike, when one is in use: as the bike is drawn when `presented`, else as the
    /// physics has it this tick (the rules' view).
    pub(super) fn bike_lens(&self, drive: &Drive, aspect: f32, presented: bool) -> Option<Lens> {
        let shot = Shot::of(self.camera_mode).filter(|_| self.view == View::Chase)?;
        let rig = drive.bike_cameras()?;
        let view = drive.bike_view(presented)?;
        let looking_back = drive.camera_looking_back(self.camera_mode);
        let (in_model, mount) = match shot {
            Shot::Front => (Vec3::from_array(rig.front.position), bike::Mount::front(&rig.front, looking_back)),
            Shot::Helmet => {
                let eye = view.eye.unwrap_or(Vec3::from_array(rig.helmet.fixed_position));
                let widen = match self.extra.camera.speed_zoom {
                    true => bike::speed_widening(drive.speed(), self.tuning.fov_intensity),
                    false => 0.0,
                };
                (eye, bike::Mount::helmet(&rig.helmet, looking_back).with_head(view.head).with_widening(widen))
            }
        };
        let eye = mount.place(view.origin(), view.rotation, in_model);
        Some(bike::lens(eye, bike::Attitude::of(view.rotation), &mount, aspect))
    }
}
