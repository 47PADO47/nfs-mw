//! What a co-op guest sees in its half of the split screen: its own chase camera and its own HUD, with the world's
//! shared blips (`docs/specs/coop-career.md`). The halves themselves are drawn by `split/view.rs`.

use super::super::WorldScene;
use crate::hud::HudState;
use crate::viewer::camera::Lens;

impl WorldScene {
    /// The first guest's camera at `aspect`; `None` without a guest or before its car is on the road.
    pub(in crate::scenes::world) fn guest_lens(&self, aspect: f32) -> Option<Lens> {
        let drive = self.players.first_guest().filter(|drive| drive.has_sim())?;
        Some(self.car_lens(drive, drive.camera(), aspect))
    }

    /// The first guest's HUD: its car's gauges and the minimap blips every player shares.
    pub(in crate::scenes::world) fn guest_hud(&self) -> Option<HudState> {
        let mut state = self.players.first_guest()?.hud_state()?;
        if let Some(traffic) = self.traffic.as_ref() {
            traffic.map_blips(&mut state.blips);
        }
        self.career.hud.apply(&mut state);
        self.race_start_blips(&mut state.blips);
        if self.race.is_none() {
            self.speed_trap_blips(&mut state.blips);
            state.blips.extend_from_slice(&self.map_blips);
        }
        state.blips.extend_from_slice(&self.mod_blips);
        Some(state)
    }
}
