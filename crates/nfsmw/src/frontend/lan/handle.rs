//! Where the game keeps its [`NetFrontend`]: the non-send resource [`LanBackend`]. The LAN screens, the session HUD, the
//! console and the metrics overlay read it; the game's `NetSession` wiring fills it. Without one (a build or a run with
//! no network), the LAN menu says so and the `net.*` commands answer that there is no network.

use super::backend::NetFrontend;

/// The game's network, as the screens see it. Non-send: the listen server's world is not `Send` (ADR 0011).
#[derive(Default)]
pub struct LanBackend {
    backend: Option<Box<dyn NetFrontend>>,
}

impl LanBackend {
    pub fn new(backend: Box<dyn NetFrontend>) -> Self {
        Self { backend: Some(backend) }
    }

    /// The network, when there is one.
    pub fn get(&mut self) -> Option<&mut dyn NetFrontend> {
        Some(self.backend.as_mut()?.as_mut())
    }

    /// The network for a read, when there is one.
    pub fn peek(&self) -> Option<&dyn NetFrontend> {
        Some(self.backend.as_ref()?.as_ref())
    }
}
