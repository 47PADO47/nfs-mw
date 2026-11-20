//! The live screen of the player's bike's instrument cluster, fed once a frame (`drive/rig/dash.rs`, `docs/modding/ride6-bike-dash.md`).

use blackbox_gfx::RenderBackend;

use super::WorldScene;
use super::bike_assist;

impl WorldScene {
    /// Paint the lead bike's cluster with its speed, gear and revs when the setting `live_dash` is on (off: the bike at rest), and keep the
    /// cost of the repaints in the trace log.
    pub(super) fn feed_dash(&mut self, renderer: &mut dyn RenderBackend) {
        let live = bike_assist::live_dash();
        let Some(drive) = self.players.lead_mut() else { return };
        if let Some(micros) = drive.feed_dash(renderer, live) {
            log::trace!("the cluster screen was repainted in {micros} us");
        }
    }
}
