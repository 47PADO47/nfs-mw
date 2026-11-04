//! The chase a rap sheet jump starts (`docs/specs/career-rapsheet-2.md` section 5.2): a milestone's or a bounty place's
//! jump clears the roads of traffic and cops, and once the car stands on the marker a chase starts at the car's heat, at
//! least the first level, its first cop 80 m ahead facing the car (`ai/pursuit/opening.rs`). A speed trap's jump only
//! moves the car.

use blackbox_gfx::RenderBackend;

use super::WorldScene;

/// The level the chase is asked at; the heat the car carries lifts it (`Police::begin`).
const JUMP_HEAT: u32 = 1;

impl WorldScene {
    /// The jump just put the car on its marker: the traffic and the cops (and a chase that runs) go, and the chase waits
    /// for the car to stand there.
    pub(in crate::scenes::world) fn arm_jump_chase(&mut self) {
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.stop_pursuit();
            traffic.clear_cars();
        }
        self.career.jump_chase = true;
    }

    /// Each frame: once the jumped car stands on its marker, the chase starts. A race or police switched off drop it.
    pub(in crate::scenes::world) fn update_jump_chase(&mut self, renderer: &mut dyn RenderBackend) {
        if !self.career.jump_chase {
            return;
        }
        if self.race.is_some() || self.extra.testing.police_off || self.traffic.is_none() {
            self.career.jump_chase = false;
            return;
        }
        let Some(drive) = self.players.lead() else { return };
        if !drive.has_sim() || drive.waiting_for_road().is_some() {
            return;
        }
        self.career.jump_chase = false;
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.set_jump_start(true);
        }
        let result = crate::scenes::world::ai::start_event_pursuit(self, renderer, JUMP_HEAT);
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.set_jump_start(false);
        }
        match result {
            Ok(line) => log::info!("career: the jump's chase starts: {line}"),
            Err(e) => log::warn!("career: the jump's chase did not start: {e}"),
        }
    }
}
