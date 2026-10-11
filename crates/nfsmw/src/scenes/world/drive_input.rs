//! Driving input and the camera view: the car takes the keys and pad each frame, and the view switches between
//! the chase camera and the free camera.

use glam::Vec3;

use super::{FLOWN_AWAY, View, WorldScene, ground, road};
use crate::input::{Action, ActionState};

impl WorldScene {
    /// Put a waiting car on the road once the area around it has loaded, then run its physics.
    pub(super) fn update_drive(&mut self, input: &ActionState, dt: f32) {
        let (Some(drive), physics) = (self.drive.as_mut(), &self.physics) else { return };
        drive.set_markers(self.markers_on);
        if let Some(request) = drive.waiting_for_road()
            && self.residency.complete()
        {
            let collision = self.residency.collision();
            let surfaces = &physics.surfaces;
            let estimate = ground::height_near(self.residency.placed(), request.near[0], request.near[1], 150.0);
            let top = estimate.unwrap_or(0.0) + 40.0;
            let found = match request.exact {
                Some(spawn) if drive.spawn(spawn, collision, surfaces) => Some(spawn),
                _ => road::find(request.near, |x, y| road::probe(collision, x, y, top)).filter(|&spawn| {
                    // `spawn` consumed the request when it failed above; ask again only on success.
                    drive.spawn(spawn, collision, surfaces)
                }),
            };
            match found {
                Some(spawn) => log::info!(
                    "{} on the road at ({:.0}, {:.0}, {:.1}), heading {:.0} degrees",
                    drive.car_name,
                    spawn.position.x,
                    spawn.position.y,
                    spawn.position.z,
                    spawn.heading.to_degrees()
                ),
                // No road in reach: the last place the car stood on one, if there is any.
                None if drive.last_good().is_some_and(|good| drive.spawn(good, collision, surfaces)) => {
                    log::info!(
                        "no road near ({:.0}, {:.0}); the car is back where it last was on a road",
                        request.near[0],
                        request.near[1]
                    );
                }
                None => {
                    log::warn!(
                        "no road within 400 m of ({:.0}, {:.0}) and none to go back to; switching to the free camera",
                        request.near[0],
                        request.near[1]
                    );
                    drive.cancel_request();
                    self.view = View::Fly;
                    self.camera.position = Vec3::new(request.near[0], request.near[1], estimate.unwrap_or(0.0) + 40.0);
                }
            }
        }
        if self.view == View::Chase {
            let loaded = self.residency.complete();
            let (collision, props) = self.residency.world_parts();
            drive.step(collision, props, &physics.surfaces, input, dt, loaded);
            drive.follow(self.residency.collision(), (input.value(Action::LookX), input.value(Action::LookY)), dt);
            return;
        }
        drive.age_effects(dt);
    }

    /// Switch between the chase camera and the free camera (the car waits while you fly).
    pub(super) fn toggle_view(&mut self) -> &'static str {
        let Some(drive) = self.drive.as_mut() else { return "not driving (use the drive command)" };
        self.cut.arm();
        match self.view {
            View::Chase => {
                let camera = drive.camera();
                let look = (drive.position() - camera.position).normalize_or_zero();
                self.camera.position = camera.position;
                self.camera.yaw = look.y.atan2(look.x);
                self.camera.pitch = look.z.clamp(-1.0, 1.0).asin();
                self.view = View::Fly;
                "free camera (the car waits); F or `freecam` to go back"
            }
            View::Fly => {
                // Back in the car: if the camera has been flown away from it, the car joins the
                // street nearest to where the camera is, facing the way the camera looks.
                let cam = self.camera.position;
                let at = drive.position();
                if (cam.x - at.x).hypot(cam.y - at.y) > FLOWN_AWAY {
                    drive.respawn_near([cam.x, cam.y], Some(self.camera.yaw));
                }
                self.view = View::Chase;
                "chase camera"
            }
        }
    }
}
