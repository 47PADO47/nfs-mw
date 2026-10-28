//! Slot management belongs to the normal My Cars scene, never to F6.

use super::*;
use crate::garage::my_cars;

impl GarageScene {
    pub(super) fn refresh_slots(&mut self) -> Result<()> {
        let slots = my_cars::slots()?;
        self.model.custom_slot_cars = slots
            .iter()
            .filter_map(|slot| Some((slot.id.clone(), my_cars::folder(slot, &self.model)?.to_owned())))
            .collect();
        self.model.custom_slots =
            slots.into_iter().map(|slot| (slot.id.clone(), my_cars::display_label(&slot, &self.model))).collect();
        self.model.custom_slot_limit = my_cars::slot_limit()?;
        Ok(())
    }

    pub(super) fn select_slot(&mut self, renderer: &mut dyn RenderBackend, id: &str) -> Result<()> {
        let slot = my_cars::slots()?.into_iter().find(|slot| slot.id == id).context("Custom car slot is missing")?;
        let folder = my_cars::folder(&slot, &self.model).context("This slot's car is not installed")?.to_owned();
        self.discover(&folder)?;
        let selection = my_cars::load(&self.dir, &self.model, id, &folder)?;
        self.show_selection(renderer, &selection)?;
        self.model.applied = selection.clone();
        self.model.draft = selection.clone();
        self.model.custom_slot = Some(id.to_owned());
        self.model.message = None;
        self.baseline = selection;
        self.baseline_slot = self.model.custom_slot.clone();
        self.refresh_unlock_error(&self.model.draft.clone());
        Ok(())
    }

    pub(super) fn persist_slot(&mut self, selection: &Selection) -> Result<()> {
        let label = self.model.selected().map_or_else(|| selection.car.clone(), my_cars::car_label);
        if let Some(id) = &self.model.custom_slot {
            let slot =
                my_cars::slots()?.into_iter().find(|slot| slot.id == *id).context("Custom car slot is missing")?;
            ensure!(
                my_cars::folder(&slot, &self.model) == Some(selection.car.as_str()),
                "Choose Add custom car to save a different vehicle model"
            );
            let label = self
                .model
                .custom_slots
                .iter()
                .find(|(slot, _)| slot == id)
                .map(|(_, label)| label.as_str())
                .unwrap_or(&label);
            my_cars::save(id, selection, label)?;
            return self.refresh_slots();
        }
        let id = my_cars::create(selection, &label)?;
        self.model.custom_slot = Some(id);
        self.refresh_slots()
    }

    pub(super) fn add_slot(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        let selection = self.model.draft.clone();
        self.validate_unlocks(&selection)?;
        self.show_selection(renderer, &selection)?;
        self.model.custom_slot = None;
        // Captures exercise the original menu flow without writing a profile or custom-car file.
        if self.save {
            self.persist_slot(&selection)?;
        }
        self.model.applied = selection.clone();
        self.baseline = selection;
        self.baseline_slot = self.model.custom_slot.clone();
        Ok(())
    }

    pub(super) fn delete_slot(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        ensure!(self.save, "Custom cars cannot be deleted in a screenshot run");
        let id = self.model.custom_slot.clone().context("Select a saved custom car to delete")?;
        my_cars::delete(&id)?;
        self.refresh_slots()?;
        self.model.custom_slot = None;
        self.baseline_slot = None;
        if let Some((id, _)) = self.model.custom_slots.first().cloned() {
            return self.select_slot(renderer, &id);
        }
        let stock = Selection { car: self.model.draft.car.clone(), ..Selection::default() };
        self.show_selection(renderer, &stock)?;
        self.model.applied = stock.clone();
        self.model.draft = stock.clone();
        self.baseline = stock;
        Ok(())
    }
}
