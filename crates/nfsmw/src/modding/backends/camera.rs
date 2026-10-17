//! `bb.camera`: the camera the player sees, read from the bridge's copy of the scene this frame. A view change is queued as a
//! [`WorldRequest`] and made on the scene after the mods have run. The game's views map one to one onto the ABI's numbers.

use std::rc::Rc;

use blackbox_modding::abi::BB_ERR_NOT_READY;
use blackbox_modding::host::{BackendResult, CameraBackend, CameraKind, CameraState};

use super::bridge::Bridge;
use super::world::WorldRequest;
use crate::settings::CameraMode;

/// The `bb.camera` port of the game: one per mod table, sharing the bridge.
pub struct CameraPort(pub(super) Rc<Bridge>);

/// The game's view a mod asked for.
pub fn mode_of(kind: CameraKind) -> CameraMode {
    match kind {
        CameraKind::Chase => CameraMode::Chase,
        CameraKind::DynamicFov => CameraMode::DynamicFov,
        CameraKind::Bumper => CameraMode::Bumper,
        CameraKind::Hood => CameraMode::Hood,
        CameraKind::Close => CameraMode::Close,
        CameraKind::Far => CameraMode::Far,
        CameraKind::SuperFar => CameraMode::SuperFar,
        CameraKind::Drift => CameraMode::Drift,
        CameraKind::Pursuit => CameraMode::Pursuit,
    }
}

/// The ABI's view for a game view.
pub fn kind_of(mode: CameraMode) -> CameraKind {
    match mode {
        CameraMode::Chase => CameraKind::Chase,
        CameraMode::DynamicFov => CameraKind::DynamicFov,
        CameraMode::Bumper => CameraKind::Bumper,
        CameraMode::Hood => CameraKind::Hood,
        CameraMode::Close => CameraKind::Close,
        CameraMode::Far => CameraKind::Far,
        CameraMode::SuperFar => CameraKind::SuperFar,
        CameraMode::Drift => CameraKind::Drift,
        CameraMode::Pursuit => CameraKind::Pursuit,
    }
}

impl CameraBackend for CameraPort {
    fn get(&self, _owner: &str) -> BackendResult<CameraState> {
        let camera = self.0.camera.get().ok_or(BB_ERR_NOT_READY)?;
        Ok(CameraState {
            kind: kind_of(camera.mode),
            position: camera.position,
            forward: camera.forward,
            fov: camera.fov,
        })
    }

    fn set_mode(&self, _owner: &str, kind: CameraKind) -> BackendResult<()> {
        if self.0.camera.get().is_none() {
            return Err(BB_ERR_NOT_READY);
        }
        self.0.world_requests.borrow_mut().push(WorldRequest::Camera(mode_of(kind)));
        Ok(())
    }
}
