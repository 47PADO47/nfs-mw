//! Pause tuning applies only after acceptance and does not create a custom-car slot.

use anyhow::{Result, ensure};
use blackbox_gfx::RenderBackend;
use nfsmw_data::car::physics::tuning::TuningCategory;

use super::WorldScene;
use crate::garage::Selection;

impl WorldScene {
    pub(in crate::scenes::world) fn garage_tuning_available(&self) -> Option<bool> {
        if self.players.is_empty() || self.split_on() {
            return None;
        }
        if self.race.as_ref().is_some_and(|race| {
            !self.career.handle.is_active() && race.kind == nfsmw_data::gameplay::RaceType::Tollbooth
        }) {
            return None;
        }
        let selection = &self.garage.applied;
        let aero = self
            .garage
            .cars
            .iter()
            .find(|car| car.folder == selection.car)
            .and_then(|car| car.parts.as_ref())
            .is_some_and(|catalog| catalog.aero_changed(&selection.parts));
        Some(TuningCategory::ALL.into_iter().any(|category| category.available(selection.performance, aero)))
    }

    pub(in crate::scenes::world) fn confirm_garage_tuning(
        &mut self,
        renderer: &mut dyn RenderBackend,
        selection: Selection,
    ) -> Result<()> {
        let mut expected =
            self.career.garage.as_ref().ok_or_else(|| anyhow::anyhow!("No tuning session is open"))?.baseline.clone();
        expected.tuning = selection.tuning;
        ensure!(expected == selection, "Performance tuning cannot purchase parts or change cars");
        if self.career.handle.is_active() {
            return self.confirm_garage_session(renderer, selection);
        }
        self.apply_garage_selection(renderer, selection)?;
        self.career.garage = None;
        self.garage.shop = false;
        self.garage.lock_car = false;
        Ok(())
    }
}
