//! `bb.world`: the track and the district the player is in, the rain and the traffic. The reads come from the bridge's copy of
//! the scene this frame; the changes are queued as [`WorldRequest`]s and made on the scene by [`apply`] after the mods have
//! run. The same queue carries the car, camera and HUD requests of `bb.vehicles`, `bb.camera` and `bb.hud`.

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_NOT_FOUND, BB_ERR_NOT_READY};
use blackbox_modding::host::{BackendResult, WorldBackend};

use super::bridge::Bridge;
use crate::hud::ModMessageTone;
use crate::settings::CameraMode;
use crate::viewer::Scene;

/// A change the mods asked for, made on the scene by [`apply`] in the order the calls came.
#[derive(Debug, Clone, PartialEq)]
pub enum WorldRequest {
    /// Put the player's car on the road nearest `at` (x forward, y left), facing `heading`.
    Move { at: [f32; 2], heading: f32 },
    /// Repair the player's car.
    Repair,
    /// Show this gameplay camera.
    Camera(CameraMode),
    /// Keep it raining (`always`), or leave it to the weather, at `amount`.
    Rain { always: bool, amount: f32 },
    /// Keep `cars` traffic cars, `cop_share` percent of them cops.
    Traffic { cars: u32, cop_share: u8 },
    /// Show a HUD message for `secs` seconds.
    Message { text: String, tone: ModMessageTone, secs: f32 },
    /// Take the HUD message down.
    HideMessage,
}

/// The `bb.world` port of the game: one per mod table, sharing the bridge.
pub struct WorldPort(pub(super) Rc<Bridge>);

impl WorldPort {
    /// Queue `request`. The scene takes it after the mods have run.
    pub(super) fn queue(&self, request: WorldRequest) {
        self.0.world_requests.borrow_mut().push(request);
    }

    /// Whether a world is loaded, as the bridge saw it this frame.
    fn loaded(&self) -> BackendResult<()> {
        if self.0.world.borrow().is_none() {
            return Err(BB_ERR_NOT_READY);
        }
        Ok(())
    }
}

/// Make the queued changes on `scene`, in order.
pub fn apply(requests: Vec<WorldRequest>, scene: &mut dyn Scene) {
    for request in requests {
        match request {
            WorldRequest::Move { at, heading } => {
                scene.move_player(at, heading);
            }
            WorldRequest::Repair => {
                scene.repair_player();
            }
            WorldRequest::Camera(mode) => scene.set_camera_mode(mode),
            WorldRequest::Rain { always, amount } => scene.set_rain(always, amount),
            WorldRequest::Traffic { cars, cop_share } => scene.set_traffic_density(cars, cop_share),
            WorldRequest::Message { text, tone, secs } => crate::hud::set_mod_message(&text, tone, secs),
            WorldRequest::HideMessage => crate::hud::hide_mod_message(),
        }
    }
}

impl WorldBackend for WorldPort {
    fn track(&self, _owner: &str) -> BackendResult<String> {
        self.0.world.borrow().as_ref().map(|world| world.track.clone()).ok_or(BB_ERR_NOT_READY)
    }

    fn section(&self, _owner: &str) -> BackendResult<String> {
        let world = self.0.world.borrow();
        let world = world.as_ref().ok_or(BB_ERR_NOT_READY)?;
        world.section.clone().ok_or(BB_ERR_NOT_FOUND)
    }

    fn set_rain(&self, _owner: &str, always: bool, amount: f32) -> BackendResult<()> {
        self.loaded()?;
        self.queue(WorldRequest::Rain { always, amount });
        Ok(())
    }

    fn set_traffic(&self, _owner: &str, cars: u32, cop_share: u32) -> BackendResult<()> {
        self.loaded()?;
        let cop_share = u8::try_from(cop_share).map_err(|_| BB_ERR_ARG)?;
        self.queue(WorldRequest::Traffic { cars, cop_share });
        Ok(())
    }
}
