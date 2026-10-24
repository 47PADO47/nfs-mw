//! Normal garage preview lifetime, separate from the developer overlay.
use anyhow::{Context, Result, ensure};
use blackbox_gfx::RenderBackend;

use super::WorldScene;
use crate::garage::Selection;

#[derive(Clone)]
pub(in crate::scenes::world) struct ShopSession {
    pub baseline: Selection,
    pub handle: Option<u32>,
    pub yaw: f32,
}

impl WorldScene {
    pub(in crate::scenes::world) fn begin_garage_session(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        ensure!(self.career.garage.is_none(), "A garage session is already open");
        ensure!(!self.players.is_empty(), "Get into a car before opening the garage");
        self.restore_career_garage(renderer)?;
        if !self.career.handle.is_active()
            && let Some(id) = self.garage.custom_slot.clone()
        {
            let slot = crate::garage::my_cars::slots()?
                .into_iter()
                .find(|slot| slot.id == id)
                .context("The driven custom car slot was deleted")?;
            let folder = crate::garage::my_cars::folder(&slot, &self.garage)
                .context("The saved custom car is not installed")?
                .to_owned();
            self.discover_garage_parts(&folder)?;
            let saved = crate::garage::my_cars::load(&self.dir, &self.garage, &id, &folder)?;
            self.apply_garage_selection(renderer, saved)?;
        }
        let handle = self
            .career
            .handle
            .with(|career| -> Result<u32> {
                let stage = career.stage();
                for unlock in nfsmw_data::car::shop_unlocks::load(&self.dir)?
                    .iter()
                    .filter(|unlock| (stage..=16).contains(&unlock.bin))
                {
                    let at = unlock.entity as usize * 8;
                    if at + 6 >= career.settings.shop_unlocks.len() {
                        continue;
                    }
                    for offset in [0, 4] {
                        let current = career.settings.shop_unlocks[at + offset] as i8;
                        if unlock.level > current {
                            career.settings.shop_unlocks[at + offset] = unlock.level as u8;
                            career.settings.shop_unlocks[at + offset + 1] = unlock.level as u8;
                        }
                    }
                }
                let handle = career.settings.car_handle;
                let car = career.stable.car_by_handle(handle).context("The current career car is unavailable")?;
                ensure!(car.is_owned(), "Only an owned career car can enter this shop");
                let entry = self
                    .garage
                    .cars
                    .iter()
                    .find(|car| car.folder == self.last_car)
                    .context("Driven car is unavailable")?;
                ensure!(
                    car.vehicle_key == blackbox_attrib::vlt_hash(&entry.physics_name.to_ascii_lowercase()),
                    "The driven car is not the current owned career car"
                );
                Ok(handle)
            })
            .transpose()?;
        self.garage.pricing = handle.map(|_| crate::garage::pricing::Pricing::load(&self.dir)).transpose()?;
        self.career.garage = Some(ShopSession { baseline: self.garage.applied.clone(), handle, yaw: 2.4 });
        self.garage.reset_draft();
        self.garage.shop = handle.is_some();
        self.garage.lock_car = true;
        self.garage.backroom = false;
        self.refresh_garage_quote(&self.garage.applied.clone());
        Ok(())
    }

    pub(in crate::scenes::world) fn preview_garage_session(
        &mut self,
        renderer: &mut dyn RenderBackend,
        selection: Selection,
    ) -> Result<()> {
        let session = self.career.garage.as_ref().context("No garage session is open")?;
        ensure!(selection.car == session.baseline.car, "This shop customizes the current car");
        self.apply_garage_selection(renderer, selection.clone())?;
        self.update_garage_backdrop(renderer, &selection)?;
        self.refresh_garage_quote(&selection);
        Ok(())
    }

    pub(in crate::scenes::world) fn cancel_garage_session(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        let Some(session) = self.career.garage.as_ref().cloned() else { return Ok(()) };
        // Keep the session alive through restoration, so rollback cannot cool or save the car.
        self.apply_garage_selection(renderer, session.baseline)?;
        self.career.garage = None;
        self.garage.shop = false;
        self.garage.lock_car = false;
        Ok(())
    }
}
