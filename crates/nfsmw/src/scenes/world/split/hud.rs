//! What each player's HUD shows in a split-screen match. The first player's is the single-player HUD (`hud.rs`); the second
//! player's gauges come from their racer's car and their race card from the engine's numbers for their racer.

use glam::Vec3;

use super::SECOND;
use crate::hud::{HudState, MapPosition};
use crate::scenes::world::WorldScene;

impl WorldScene {
    /// The HUD of one player (0 the top half, 1 the bottom). `None` outside a split-screen match.
    pub(in crate::scenes::world) fn split_hud(&self, player: usize) -> Option<HudState> {
        self.split.as_ref()?;
        match player {
            0 => self.hud_frame(),
            _ => self.second_hud(),
        }
    }

    fn second_hud(&self) -> Option<HudState> {
        let traffic = self.traffic.as_ref()?;
        let mut state = traffic.racer_hud(SECOND)?;
        state.race = self.race_hud_for(SECOND);
        state.leaders = self.race_leaders_for(SECOND);
        state.message = self.race_message_for(SECOND);
        state.drag = self.race.as_ref().is_some_and(|r| r.kind == nfsmw_data::gameplay::RaceType::Drag);
        state.map_source = self.race.as_ref().map(|r| r.map.clone()).unwrap_or_default();
        self.race_blips(&mut state.blips);
        Some(state)
    }
}

/// The minimap place of a car: where it is and which way it points.
pub(in crate::scenes::world) fn map_position(position: Vec3, nose: Vec3) -> MapPosition {
    MapPosition {
        position: [position.x, position.y],
        heading: [nose.x, nose.y],
        orientation: blackbox_minimap::Orientation::North,
    }
}
