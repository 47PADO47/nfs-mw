//! Normal checkout changes the save. Developer overlay changes never enter here.
use anyhow::{Context, Result, ensure};
use blackbox_gfx::RenderBackend;
use nfsmw_data::car::CarTables;

use super::{WorldScene, persistence, records};
use crate::garage::Selection;

impl WorldScene {
    pub(in crate::scenes::world) fn confirm_garage_session(
        &mut self,
        renderer: &mut dyn RenderBackend,
        selection: Selection,
    ) -> Result<()> {
        let session = self.career.garage.as_ref().cloned().context("No garage session is open")?;
        let quote = self.garage_quote(&selection)?;
        // Complete all fallible assembly before charging or touching the native record.
        self.preview_garage_session(renderer, selection.clone())?;
        let Some(handle) = session.handle else {
            let id = crate::garage::my_cars::save_world_slot(&selection, self.garage.custom_slot.as_deref())?;
            self.garage.custom_slot = Some(id);
            self.career.garage = None;
            self.garage.shop = false;
            self.garage.lock_car = false;
            return Ok(());
        };
        let profiles = self.career.profiles.clone().context("No career profile folder is available")?;
        let tables = CarTables::load(&self.dir)?;
        let record = self
            .career
            .handle
            .with(|career| career.stable.customization_for_handle(handle))
            .flatten()
            .context("The current car has no customization record")?;
        let record = records::write(&record, &tables, &selection)?;
        let snapshot = self
            .career
            .handle
            .with(|career| (career.settings.clone(), career.stable.clone(), career.markers.clone()))
            .context("The career is no longer active")?;
        self.career
            .handle
            .with(|career| -> Result<()> {
                ensure!(career.settings.car_handle == handle, "The current career car changed");
                let mut settings = career.settings.clone();
                let mut markers = career.markers.clone();
                let mut stable = career.stable.clone();
                nfsmw_career::economy::shop::checkout(&mut settings, &mut markers, &quote)?;
                ensure!(stable.set_customization_for_handle(handle, &record), "Cannot store this car's customization");
                (career.settings, career.stable, career.markers) = (settings, stable, markers);
                Ok(())
            })
            .context("The career is no longer active")??;
        self.garage.applied = session.baseline.clone();
        self.career_garage_cooling(&selection);
        self.garage.applied = selection.clone();
        let saved = self
            .career
            .handle
            .with(|career| -> Result<()> {
                let native = career.profile_bytes();
                persistence::save_purchase(&profiles, &career.alias, handle, &record, &selection, &native)?;
                Ok(())
            })
            .context("The career is no longer active")?;
        if let Err(error) = saved {
            self.career.handle.with(|career| {
                (career.settings, career.stable, career.markers) = snapshot;
            });
            // Keep the preview visible and the cart open so the player can retry or discard.
            return Err(error).context("Garage purchase was not saved; cash and installed parts were restored");
        }
        self.career.garage = None;
        self.garage.shop = false;
        self.garage.lock_car = false;
        self.garage.shop_error = None;
        Ok(())
    }
}
