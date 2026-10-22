//! Discover installed vehicles and apply garage requests without modifying installed assets.

use anyhow::{Context, Result, ensure};
use blackbox_gfx::RenderBackend;
use game_install::GameDir;
use nfsmw_data::car::mods as car_mods;
use nfsmw_data::car::{
    CarTables, Paint, customization,
    physics::{PerformanceCatalog, PhysicsData},
    slot, stock_parts,
};

use super::WorldScene;
use crate::garage::{Car, Category, Model, Selection};

pub(super) mod command;
mod names;

pub(super) fn catalog(dir: &GameDir, physics: &PhysicsData) -> Result<Model> {
    let tables = CarTables::load(dir)?;
    let mut cars = Vec::new();
    let brands: Vec<_> = tables.types.iter().map(|car| car.manufacturer.clone()).collect();
    let strings =
        nfsmw_data::shared::language(dir).ok().and_then(|bytes| blackbox_text::StringTable::from_file(&bytes).ok());
    for folder in nfsmw_data::car::list(dir) {
        // Shared accessory folders have geometry but are not vehicle types.
        let Some(car_type) = tables.car_type_for_folder(&folder) else { continue };
        let Some(category) = Category::from_usage(car_type.usage_type) else { continue };
        // A mod's cop-only car is not a player car (car/mods.rs).
        if !car_mods::is_player_car(&folder) {
            continue;
        }
        let Ok(data) = physics.car(&car_type.type_name) else { continue };
        if data.spec.engine.torque.len() < 2
            || !data.spec.engine.torque.iter().any(|t| *t > 0.0)
            || data.spec.engine.red_line <= data.spec.engine.idle
            || data.spec.transmission.gear_ratio.len() <= 2
            || data.spec.transmission.gear_ratio[2] <= 0.0
        {
            continue;
        }
        let (mut name, manufacturer) =
            names::display(&car_type.type_name, &car_type.manufacturer, &brands, strings.as_ref());
        if let Some(own) = car_mods::display_name(&folder) {
            name = own.to_owned();
        }
        let original_color = stock_parts(&tables, car_type)[slot::BASE_PAINT].map(|part| Paint::of(&tables, &part).rgb);
        cars.push(Car {
            folder,
            name,
            manufacturer,
            category,
            original_color,
            paintable: car_type.skinnable,
            physics_name: car_type.type_name.clone(),
            performance: PerformanceCatalog::read(physics.database(), &car_type.type_name)?,
            parts: None,
            unavailable: None,
        });
    }
    cars.sort_by(|a, b| (&a.manufacturer, &a.name).cmp(&(&b.manufacturer, &b.name)));
    Ok(Model { cars, ..Model::default() })
}

#[cfg(test)]
mod customization_tests;
#[cfg(test)]
mod tests;

impl WorldScene {
    pub(super) fn discover_garage_parts(&mut self, folder: &str) -> Result<()> {
        let Some(car) = self.garage.cars.iter_mut().find(|car| car.folder == folder) else { return Ok(()) };
        if car.parts.is_some() {
            return Ok(());
        }
        if self.garage_parts.is_none() {
            self.garage_parts = Some(customization::Inventory::load(&self.dir)?);
        }
        let mut catalog =
            self.garage_parts.as_ref().context("part inventory unavailable")?.catalog(&self.dir, folder)?;
        let strings = nfsmw_data::shared::language(&self.dir)
            .ok()
            .and_then(|bytes| blackbox_text::StringTable::from_file(&bytes).ok());
        for (index, choices) in catalog.choices.iter_mut().enumerate() {
            for choice in choices.iter_mut() {
                if index != customization::Category::Body as usize
                    && index != customization::Category::Wheels as usize
                    && let Some(name) = choice.language_hash.and_then(|h| strings.as_ref()?.get(h))
                    && !name.trim().is_empty()
                {
                    choice.label = name;
                }
                if choice.carbon {
                    choice.label.push_str(" (Carbon)");
                }
            }
            choices.sort_by(|a, b| (&a.label, a.hash).cmp(&(&b.label, b.hash)));
            // Some installed variants share a localized name. Keep every identity selectable.
            let labels: Vec<_> = choices.iter().map(|p| p.label.clone()).collect();
            for (i, choice) in choices.iter_mut().enumerate() {
                let duplicate = labels.iter().filter(|label| **label == labels[i]).count() > 1;
                if duplicate {
                    let variant = labels[..=i].iter().filter(|label| **label == labels[i]).count();
                    choice.label.push_str(&format!(" · {variant}"));
                }
            }
        }
        car.parts = Some(catalog);
        Ok(())
    }

    pub(super) fn apply_garage_selection(
        &mut self,
        renderer: &mut dyn RenderBackend,
        selection: Selection,
    ) -> Result<()> {
        self.discover_garage_parts(&selection.car)?;
        let entry =
            self.garage.cars.iter().find(|car| car.folder == selection.car).context("car is not in the garage")?;
        ensure!(entry.unavailable.is_none(), "{}", entry.unavailable.as_deref().unwrap_or_default());
        entry.parts.as_ref().context("customization catalog unavailable")?.validate(&selection.parts)?;
        selection.gauges.validate(entry.parts.as_ref().context("customization catalog unavailable")?)?;
        let tunings = selection.tuning.normalized()?;
        let changed_performance = selection.performance != self.garage.applied.performance;
        let changed_parts = selection.parts != self.garage.applied.parts;
        let physics = match changed_performance || selection.car != self.last_car {
            true => Some(self.physics.car_with_performance(&entry.physics_name, selection.performance)?),
            false => None,
        };
        if selection.car == self.last_car
            && !changed_parts
            && selection.color == self.garage.applied.color
            && let Some(drive) = self.players.lead_mut()
        {
            if let Some(physics) = physics {
                drive.set_performance(physics, selection.performance);
            }
            self.commit_garage_selection(selection, tunings);
            return Ok(());
        }
        if selection.car == self.last_car
            && !changed_parts
            && let Some(rgb) = selection.color
            && let Some(drive) = self.players.lead_mut()
        {
            drive.recolor(renderer, rgb, self.sun.to_sun())?;
            if let Some(physics) = physics {
                drive.set_performance(physics, selection.performance);
            }
            self.commit_garage_selection(selection, tunings);
            self.reflection_last = None;
            return Ok(());
        }
        let folder = selection.car.clone();
        let mut model = nfsmw_data::car::load_customized_driving(
            &self.dir,
            &folder,
            &nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None },
            &selection.parts,
        )?;
        if let Some(color) = selection.color {
            model.set_body_color(color)?;
        }
        // Loading, paint validation and physics validation precede replacing the active rig.
        self.physics_of(&model)?;
        if folder == self.last_car
            && let Some(drive) = self.players.lead_mut()
        {
            drive.replace_model(renderer, model, self.car_shading, self.sun.to_sun());
            if let Some(physics) = physics {
                drive.set_performance(physics, selection.performance);
            }
            self.commit_garage_selection(selection, tunings);
            self.reflection_last = None;
            return Ok(());
        }
        self.start_driving(renderer, folder, model, None)?;
        if let Some(drive) = self.players.lead_mut()
            && let Some(physics) = physics
        {
            drive.set_performance(physics, selection.performance);
        }
        self.commit_garage_selection(selection, tunings);
        self.prepare_drive();
        self.reflection_last = None;
        Ok(())
    }

    fn commit_garage_selection(&mut self, selection: Selection, tunings: blackbox_vehicle::Tunings) {
        // The parts this fits cool the career car's heat (`career/cooling.rs`), compared with what was applied before.
        self.career_garage_cooling(&selection);
        if let Some(drive) = self.players.lead_mut() {
            drive.set_tuning(selection.tuning, tunings);
        }
        self.garage.applied = selection.clone();
        self.garage.draft = selection;
    }
}
