//! The HUD of the world scene: the drive's state, with the traffic's and the map's blips on the minimap.

use super::{View, WorldScene};

impl WorldScene {
    /// The HUD for this frame. Hidden in the fly view and while there is no car yet.
    pub(super) fn hud_frame(&self) -> Option<crate::hud::HudState> {
        let hidden = Some(crate::hud::HudState { visible: false, ..Default::default() });
        if self.view == View::Fly {
            return hidden;
        }
        // No car yet (the world is still loading): hidden, not the idle HUD a viewer shows.
        let mut hud = self.drive.as_ref().and_then(|drive| drive.hud_state()).or(hidden);
        if let (Some(state), Some(traffic)) = (hud.as_mut(), self.traffic.as_ref()) {
            traffic.map_blips(&mut state.blips);
            state.blips.extend_from_slice(&self.map_blips);
            // The radar's green laser shows while cops chase the player.
            state.cop_nearby = traffic.is_pursuing();
        }
        hud
    }
}
