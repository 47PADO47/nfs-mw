//! Driving input and the camera view: the car takes the keys and pad each frame, and the view switches between
//! the chase camera and the free camera.

use glam::Vec3;

use super::drive::Strict;
use super::road::Spawn;
use super::{FLOWN_AWAY, View, WorldScene, ground, road};

/// How long a car may wait for its surroundings to load before it is placed on the nearest road anyway.
const ROAD_WAIT: std::time::Duration = std::time::Duration::from_secs(10);
/// How far above a car reset where it stands the road under it is looked for.
const RESET_ABOVE: f32 = 3.0;

impl WorldScene {
    /// Put a waiting car on the road once the area around it has loaded; the physics runs in `update_simulation`.
    pub(super) fn prepare_drive(&mut self) {
        self.reset_onto_race_route();
        let (Some(drive), physics) = (self.players.lead_mut(), &self.physics) else { return };
        drive.set_markers(self.markers_on);
        let Some(request) = drive.waiting_for_road() else {
            self.waiting_since = None;
            return;
        };
        let waited = self.waiting_since.get_or_insert_with(std::time::Instant::now).elapsed();
        let loaded = self.residency.complete();
        let timed_out = waited > ROAD_WAIT;
        if !loaded && !timed_out {
            return;
        }
        let collision = self.residency.collision();
        let surfaces = &physics.surfaces;
        // A race start waits for its own ground and is never moved to another road.
        if request.strict
            && let Some(spawn) = request.exact
        {
            match drive.place_strict(spawn, collision, surfaces) {
                Strict::Waiting => {}
                Strict::Placed => {
                    self.waiting_since = None;
                    log_placed(&drive.car_name, spawn);
                }
                Strict::Lost => log::warn!(
                    "no ground at the start ({:.0}, {:.0}, {:.1}) after {:.0} s; looking for the nearest road",
                    spawn.position.x,
                    spawn.position.y,
                    spawn.position.z,
                    waited.as_secs_f32()
                ),
            }
            return;
        }
        if !loaded {
            log::warn!(
                "the world did not finish loading after {:.0} s; placing the car on the nearest road it can reach",
                waited.as_secs_f32()
            );
        }
        self.waiting_since = None;
        let estimate = ground::height_near(self.residency.placed(), request.near[0], request.near[1], 150.0);
        // A car reset where it stands looks for a road from just above itself first, so not on a bridge over it.
        let at = drive.position();
        let own = (drive.has_sim() && (at.x - request.near[0]).hypot(at.y - request.near[1]) < 1.0).then_some(at.z);
        let tops = [own.map(|z| z + RESET_ABOVE), Some(estimate.unwrap_or(0.0) + 40.0)];
        let found = match request.exact {
            Some(spawn) if drive.spawn(spawn, collision, surfaces) => Some(spawn),
            _ => tops.into_iter().flatten().find_map(|top| {
                road::find(request.near, |x, y| road::probe(collision, x, y, top)).filter(|&spawn| {
                    // `spawn` consumed the request when it failed above; ask again only on success.
                    drive.spawn(spawn, collision, surfaces)
                })
            }),
        };
        match found {
            Some(spawn) => log_placed(&drive.car_name, spawn),
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

    /// Switch between the chase camera and the free camera (the car waits while you fly).
    pub(super) fn toggle_view(&mut self) -> &'static str {
        let original = crate::viewer::camera::original::index_of(self.camera_mode)
            .map(|_| self.gameplay_lens(self.physics_aspect));
        let Some(drive) = self.players.lead_mut() else { return "not driving (use the drive command)" };
        self.cut.arm();
        self.original_camera.reset();
        self.original_player_step = None;
        match self.view {
            View::Chase => {
                let camera = drive.camera();
                let look = original
                    .map_or_else(|| (drive.position() - camera.position).normalize_or_zero(), |lens| lens.forward);
                self.camera.position = original.map_or(camera.position, |lens| lens.position);
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

fn log_placed(car: &str, spawn: Spawn) {
    log::info!(
        "{car} on the road at ({:.0}, {:.0}, {:.1}), heading {:.0} degrees",
        spawn.position.x,
        spawn.position.y,
        spawn.position.z,
        spawn.heading.to_degrees()
    );
}
