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

mod camera;
mod checkout;
pub(super) mod command;
mod names;
mod pause_tuning;
mod persistence;
mod quote;
mod records;
pub(super) mod session;
mod showroom;

pub(crate) fn catalog(dir: &GameDir, physics: &PhysicsData) -> Result<Model> {
    let tables = CarTables::load(dir)?;
    let ratings = nfsmw_data::car::physics::ratings::RatingBook::load(dir)?;
    let mut frontend = physics.database().clone();
    frontend.load(&nfsmw_data::read_unwrapped(dir, nfsmw_data::career_cars::FE_ATTRIB_FILE)?)?;
    let mut cars = Vec::new();
    let brands: Vec<_> = tables.types.iter().map(|car| car.manufacturer.clone()).collect();

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
        let (mut name, mut manufacturer) =
            names::display(&car_type.type_name, &car_type.manufacturer, &brands, &crate::i18n::game);
        if let Some(own) = car_mods::display_name(&folder) {
            name = own.to_owned();
        }
        // The brand a mod gives is shown as it is written; without one the clone's brand stays.
        if let Some(brand) = car_mods::manufacturer(&folder) {
            manufacturer = brand.to_owned();
        }
        let original_color = stock_parts(&tables, car_type)[slot::BASE_PAINT].map(|part| Paint::of(&tables, &part).rgb);
        let performance = PerformanceCatalog::read(physics.database(), &car_type.type_name)?;
        let performance_details = nfsmw_data::car::physics::performance::PerformanceDetails::read(
            &frontend,
            &car_type.type_name,
            &performance,
            data.spec.induction.kind() == blackbox_vehicle::induction::InductionKind::Turbo,
        );
        cars.push(Car {
            folder,
            name,
            manufacturer,
            category,
            original_color,
            paintable: car_type.skinnable,
            hero: car_type.index == 0x29
                && tables.stock_types.iter().any(|native| native.type_name_hash == car_type.type_name_hash),
            physics_name: car_type.type_name.clone(),
            performance,
            performance_details,
            performance_ratings: ratings.for_car(&car_type.type_name),
            parts: None,
            unavailable: None,
        });
    }
    cars.sort_by(|a, b| (&a.manufacturer, &a.name).cmp(&(&b.manufacturer, &b.name)));
    Ok(Model { cars, ..Model::default() })
}

/// Imported original My Cars records retain their exact handle identity.
pub(crate) fn native_my_car_handle(
    dir: &GameDir,
    model: &Model,
    folder: &str,
    handle: u32,
) -> Result<Option<Selection>> {
    let entry = model.cars.iter().find(|entry| entry.folder == folder).context("Car is not installed")?;
    let vehicle_key = blackbox_attrib::vlt_hash(&entry.physics_name.to_ascii_lowercase());
    let record = crate::career::CareerHandle::shared()
        .with_loaded(|career| {
            let car = career
                .stable
                .cars
                .iter()
                .find(|car| car.is_quick_race() && car.vehicle_key == vehicle_key && car.handle == handle)?;
            career.stable.customization_for_handle(car.handle)
        })
        .flatten();
    let Some(record) = record else { return Ok(None) };
    Ok(Some(records::read(&record, &CarTables::load(dir)?, folder, model)?))
}

#[cfg(test)]
mod checkout_tests;
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
        for (index, choices) in catalog.choices.iter_mut().enumerate() {
            for choice in choices.iter_mut() {
                if index != customization::Category::Body as usize
                    && index != customization::Category::Wheels as usize
                    && let Some(name) = choice.language_hash.and_then(crate::i18n::game)
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
        car.parts = Some(catalog.into());
        Ok(())
    }

    pub(super) fn apply_garage_selection(
        &mut self,
        renderer: &mut dyn RenderBackend,
        selection: Selection,
    ) -> Result<()> {
        self.discover_garage_parts(&selection.car)?;
        self.garage.castrol_unlocked = crate::career::CareerHandle::shared()
            .with_loaded(|career| career.settings.flags.contains(nfsmw_career::settings::Flags::BONUS_CAR_0X34498EB2))
            .unwrap_or(false);
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
        if let Some(physics) = &physics {
            self.garage.gauge_max_rpm = physics.spec.engine.max_rpm;
            self.garage.gauge_has_turbo =
                physics.spec.induction.kind() != blackbox_vehicle::induction::InductionKind::None;
        }
        if physics.is_none()
            && let Some(hud) = self.players.lead().and_then(|drive| drive.hud_state())
        {
            self.garage.gauge_max_rpm = hud.max_rpm;
            self.garage.gauge_has_turbo = hud.has_turbo;
        }
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
        // Comparison placement is useful only after the player explicitly detaches the view.
        // A normal car swap must never create a fixed camera bookmark.
        let comparing = self.extra.freecam || self.extra.selected_bookmark.is_some();
        let preserved = self
            .players
            .lead()
            .filter(|_| self.extra.camera.preserve_framing && comparing)
            .and_then(|drive| drive.last_good());
        self.start_driving_setup(renderer, folder, model, None, false)?;
        if let (Some(drive), Some(spawn)) = (self.players.lead_mut(), preserved) {
            drive.respawn_at(spawn);
        }
        if let Some(drive) = self.players.lead_mut()
            && let Some(physics) = physics
        {
            drive.set_performance(physics, selection.performance);
        }
        self.commit_garage_selection(selection, tunings);
        self.prepare_drive();
        if self.extra.freecam {
            self.view = super::View::Fly;
        }
        self.reflection_last = None;
        Ok(())
    }

    fn commit_garage_selection(&mut self, selection: Selection, tunings: blackbox_vehicle::Tunings) {
        // Applying is a preview or a developer experiment. Only normal checkout changes the career.
        if let Some(drive) = self.players.lead_mut() {
            drive.set_tuning(selection.tuning, tunings);
        }
        self.garage.applied = selection.clone();
        self.garage.draft = selection;
    }
}
