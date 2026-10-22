//! Where the game keeps its [`NetFrontend`]: the non-send resource [`LanBackend`]. The LAN screens, the session HUD, the
//! console and the metrics overlay read it; the game's `NetSession` wiring fills it
//! (`app.insert_non_send_resource(LanBackend::new(Box::new(session)))`). Without one, the LAN menu runs on a network that
//! refuses everything (`offline.rs`), or on the pretend session of `demo.rs` with `NFSMW_LAN_DEMO=1`.

use super::backend::NetFrontend;

/// The game's network, as the screens see it. Non-send: the listen server's world is not `Send` (ADR 0011).
#[derive(Default)]
pub struct LanBackend {
    backend: Option<Box<dyn NetFrontend>>,
}

impl LanBackend {
    #[allow(dead_code)] // called by the game's `NetSession` wiring (`crate::net`)
    pub fn new(backend: Box<dyn NetFrontend>) -> Self {
        Self { backend: Some(backend) }
    }

    /// The network, when there is one.
    pub fn get(&mut self) -> Option<&mut dyn NetFrontend> {
        Some(self.backend.as_mut()?.as_mut())
    }
}
