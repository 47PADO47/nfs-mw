//! The driven car's live heat in the world (`docs/specs/career-safehouse-2.md` section 8). The world sets it once from the
//! car's career record when the car is made (`max(1.0, record)`), the chases change it, and the world's teardown writes it
//! back to the record (`min(live, 5.0)`, the player car's destructor). In between, the record and the live heat are apart:
//! the evade, race-win and milestone factors multiply the records only, and the next chase starts from the live heat.

use nfsmw_career::safehouse::heat::unload_world;
use nfsmw_career::safehouse::{SPAWN_HEAT_FLOOR, spawn_heat};

use crate::scenes::world::WorldScene;

/// Which cars the live heat came from, and whether the teardown has written it back.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(in crate::scenes::world) struct LiveHeat {
    /// The lead's car and the co-op guest's car whose records the live heat was loaded from; `None` until the first load,
    /// and again after a record was changed under the live heat (`reload_live_heat`).
    loaded_for: Option<(u32, Option<u32>)>,
    /// The live heat was written back for this teardown (the safe house writes it while its chase still runs).
    written: bool,
}

impl LiveHeat {
    /// The cars the live heat must be loaded for now: `None` when it already was (the same cars), else the cars it was
    /// loaded for before (written back to the old lead's record first) and the new ones.
    fn due(&self, cars: (u32, Option<u32>)) -> Option<Option<(u32, Option<u32>)>> {
        (self.loaded_for != Some(cars)).then_some(self.loaded_for)
    }
}

/// The heat a chase starts from: the lead car's heat, or the higher of it and the guest car's when the guest has a
/// record [ours].
fn carried(lead: f32, guest: Option<f32>) -> f32 {
    guest.map_or(lead, |guest| lead.max(guest))
}

impl WorldScene {
    /// Loads the cars' record heat into the police when the car is new to the world: the first frame without a chase,
    /// and when the current car changes (the old car's record takes the live heat first, as a world teardown would).
    pub(super) fn load_live_heat(&mut self) {
        let guest_handle = self.coop.as_ref().and_then(|session| session.guest_handle);
        let Some(lead) = self.career.handle.with(|career| career.settings.car_handle) else { return };
        let Some(previous) = self.career.live_heat.due((lead, guest_handle)) else { return };
        if let Some((old, _)) = previous.filter(|(old, _)| *old != lead) {
            self.write_heat_to(old);
        }
        let heat = self.career.handle.with(|career| {
            let lead = spawn_heat(&career.settings, &career.stable, true);
            let guest = guest_handle
                .and_then(|handle| career.stable.career_for_handle(handle))
                .map(|record| record.heat.max(SPAWN_HEAT_FLOOR));
            carried(lead, guest)
        });
        let (Some(heat), Some(traffic)) = (heat, self.traffic.as_mut()) else { return };
        traffic.set_carried_heat(heat);
        self.career.live_heat = LiveHeat { loaded_for: Some((lead, guest_handle)), written: false };
        log::info!("career: the car starts with heat {heat:.2}");
    }

    /// The driven car's live heat: the chase's while one runs, else the heat it carries. `None` without traffic or before
    /// the heat was loaded.
    pub(super) fn live_heat(&self) -> Option<f32> {
        self.career.live_heat.loaded_for?;
        Some(self.traffic.as_ref()?.vehicle_heat())
    }

    /// The world is being torn down: the driven car's live heat goes into its career record (`heat::unload_world`,
    /// capped at 5), once. The safe house writes it while its chase still runs; the scene's close writes it otherwise.
    pub(in crate::scenes::world) fn write_driven_heat(&mut self) {
        if !self.career.handle.is_active() || self.career.live_heat.written {
            return;
        }
        let Some(lead) = self.career.handle.with(|career| career.settings.car_handle) else { return };
        if self.write_heat_to(lead) {
            self.career.live_heat.written = true;
        }
    }

    /// A record heat is about to change under the live heat while the world stays (the garage cools the car it fits
    /// parts to): the live heat is written back first, and the car starts again from the changed record, as the original's
    /// shop round trip through a new world does.
    pub(in crate::scenes::world) fn reload_live_heat(&mut self) {
        self.write_driven_heat();
        self.career.live_heat = LiveHeat::default();
    }

    /// Writes `min(live heat, 5.0)` into the record of the car `handle`; false when there is no live heat or no record.
    fn write_heat_to(&mut self, handle: u32) -> bool {
        let Some(heat) = self.live_heat() else { return false };
        let written = self.career.handle.with(|career| {
            let mut settings = career.settings.clone();
            settings.car_handle = handle;
            unload_world(&settings, &mut career.stable, heat, true)
        });
        if written != Some(true) {
            log::warn!("career: the driven car has no career record; its heat is not kept");
            return false;
        }
        log::info!("career: the car's heat {heat:.2} is written to its record");
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_heat_loads_once_per_car_and_again_for_another_car() {
        let mut live = LiveHeat::default();
        assert_eq!(live.due((7, None)), Some(None), "the first frame loads it");
        live.loaded_for = Some((7, None));
        assert_eq!(live.due((7, None)), None, "the same car keeps its live heat");
        assert_eq!(live.due((9, None)), Some(Some((7, None))), "another car loads, after the old one's is written");
        assert_eq!(live.due((7, Some(3))), Some(Some((7, None))), "a co-op guest joining loads again");
    }

    #[test]
    fn a_chase_starts_from_the_hotter_of_the_two_cars() {
        assert_eq!(carried(2.0, None), 2.0);
        assert_eq!(carried(2.0, Some(3.5)), 3.5);
        assert_eq!(carried(4.0, Some(1.0)), 4.0);
    }
}
