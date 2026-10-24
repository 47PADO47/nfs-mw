//! Normal My Cars uses quick-race unlock fields. The diagnostic overlay has its own model.

use super::*;
use crate::career::CareerHandle;
use nfsmw_career::settings::{CareerSettings, Flags};
use nfsmw_data::car::{
    CarTables,
    customization::{Category, DecalZone},
    physics::PerformanceCategory,
};

fn allowed(settings: &CareerSettings, completed: bool, entity: u32, level: u32, label: &str) -> Result<()> {
    if level == 0 || completed || settings.flags.contains(Flags::CAREER_COMPLETE) {
        return Ok(());
    }
    let available = nfsmw_career::economy::shop::unlocked_level(settings, entity, true).unwrap_or(-1);
    if i64::from(level) > i64::from(available) {
        return Err(anyhow::Error::new(nfsmw_career::economy::shop::ShopError::Locked { entity, level }))
            .with_context(|| format!("{label} level {level} is locked in My Cars"));
    }
    Ok(())
}

impl GarageScene {
    pub(super) fn gate_cars(&mut self) {
        let pool = crate::frontend::quick_play::pool::load_now(&self.dir);
        let progress = crate::frontend::custom_race::car_select::progress();
        let entries = crate::frontend::custom_race::car_list::entries(
            crate::frontend::custom_race::car_list::List::Stock,
            &pool,
            &progress,
        );
        for car in &mut self.model.cars {
            if entries.iter().any(|entry| entry.car.folder.eq_ignore_ascii_case(&car.folder) && entry.locked) {
                car.unavailable = Some("Unlock this car by progressing through the Blacklist".into());
            }
        }
    }

    pub(super) fn refresh_unlock_error(&mut self, selection: &Selection) {
        let error = self.validate_unlocks(selection).err();
        self.model.selection_locked = error.as_ref().is_some_and(|error| {
            matches!(
                error.downcast_ref::<nfsmw_career::economy::shop::ShopError>(),
                Some(nfsmw_career::economy::shop::ShopError::Locked { .. })
            )
        });
        self.model.shop_error = error.map(|error| format!("{error:#}"));
        self.model.maximum_regular = maximum_regular(&self.dir, &self.model, selection).ok();
    }

    pub(super) fn validate_unlocks(&self, selection: &Selection) -> Result<()> {
        validate_selection(&self.dir, &self.model, &self.baseline, selection)
    }
}

pub(crate) fn validate_selection(
    dir: &GameDir,
    model: &Model,
    baseline: &Selection,
    selection: &Selection,
) -> Result<()> {
    let entry = model.cars.iter().find(|car| car.folder == selection.car).context("Car is not installed")?;
    ensure!(entry.unavailable.is_none(), "{}", entry.unavailable.as_deref().unwrap_or_default());
    let (settings, completed) = quick_settings(dir)?;
    let old = baseline;
    let tables = CarTables::load(dir)?;
    for category in Category::ALL {
        if old.car == selection.car && old.parts.parts[category as usize] == selection.parts.parts[category as usize] {
            continue;
        }
        let level = selection.parts.parts[category as usize]
            .and_then(|hash| tables.parts.by_name_hash(hash))
            .map_or(0, |part| u32::from(part.upgrade_level));
        allowed(&settings, completed, nfsmw_data::car::shop::visual_entity(category), level, category.label())?;
    }
    for category in PerformanceCategory::ALL {
        let selected = selection.performance.get(category);
        if old.car != selection.car || old.performance.get(category) != selected {
            let level = nfsmw_data::car::shop::global_level(
                category,
                entry.performance.categories[category as usize].max_level,
                selected,
            )?;
            allowed(
                &settings,
                completed,
                nfsmw_data::car::shop::performance_entity(category),
                level,
                category.label(),
            )?;
        }
        if selection.performance.has_junkman(category)
            && (old.car != selection.car || !old.performance.has_junkman(category))
        {
            ensure!(
                settings.flags.contains(Flags::QUICK_RACE_JUNKMAN)
                    || (completed && !entry.folder.eq_ignore_ascii_case("BMWM3GTRE46")),
                "Unlock Junkman parts before fitting them in My Cars"
            );
        }
    }
    for zone in DecalZone::ALL {
        for position in 0..8 {
            if old.car == selection.car
                && old.parts.decals[zone as usize][position] == selection.parts.decals[zone as usize][position]
            {
                continue;
            }
            let level = selection.parts.decals[zone as usize][position]
                .and_then(|hash| tables.parts.by_name_hash(hash))
                .map_or(0, |part| u32::from(part.upgrade_level));
            let entity = match zone {
                DecalZone::FrontWindow | DecalZone::RearWindow => 44,
                DecalZone::LeftDoor | DecalZone::RightDoor if position >= 6 => 43,
                DecalZone::LeftDoor | DecalZone::RightDoor => 46,
                _ => 48,
            };
            allowed(&settings, completed, entity, level, zone.label())?;
        }
    }
    if old.car != selection.car || old.gauges.skin != selection.gauges.skin {
        let level = match selection.gauges.skin {
            0 => 0,
            skin => tables
                .parts
                .parts()
                .iter()
                .find(|part| {
                    part.part_id == 82
                        && tables
                            .parts
                            .attributes(part)
                            .any(|(key, value)| key == 0x3111_51f8 && value == u32::from(skin))
                })
                .map(|part| u32::from(part.upgrade_level))
                .context("Installed gauge unlock tier is unavailable")?,
        };
        allowed(&settings, completed, 17, level, "Gauge style")?;
    }
    Ok(())
}

fn quick_settings(dir: &GameDir) -> Result<(CareerSettings, bool)> {
    let mut settings =
        CareerHandle::shared().with_loaded(|career| career.settings.clone()).unwrap_or_else(CareerSettings::new_career);
    let stage = CareerHandle::shared().with_loaded(|career| career.stage()).unwrap_or(16);
    let completed = CareerHandle::shared().with_loaded(|career| career.career_completed_once()).unwrap_or(false);
    for unlock in nfsmw_data::car::shop_unlocks::load(dir)?.iter().filter(|unlock| (stage..=16).contains(&unlock.bin)) {
        let at = unlock.entity as usize * 8 + 4;
        if let Some(value) = settings.shop_unlocks.get_mut(at)
            && unlock.level > *value as i8
        {
            *value = unlock.level as u8;
        }
    }
    Ok((settings, completed))
}

fn maximum_regular(
    dir: &GameDir,
    model: &Model,
    selection: &Selection,
) -> Result<nfsmw_data::car::physics::PerformancePackage> {
    let (settings, completed) = quick_settings(dir)?;
    let car = model.cars.iter().find(|car| car.folder == selection.car).context("Car is not installed")?;
    let mut package = selection.performance;
    for category in PerformanceCategory::ALL {
        let maximum = car.performance.categories[category as usize].max_level;
        for local in (0..=maximum).rev() {
            let global = nfsmw_data::car::shop::global_level(category, maximum, local)?;
            let entity = nfsmw_data::car::shop::performance_entity(category);
            if allowed(&settings, completed, entity, global, category.label()).is_ok() {
                package.levels[category as usize] = local;
                break;
            }
        }
    }
    Ok(package)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn my_cars_reads_quick_race_unlocks_independently_of_career_unlocks() {
        let mut settings = CareerSettings::new_career();
        let entity = 8;
        settings.shop_unlocks[entity * 8] = 3;
        settings.shop_unlocks[entity * 8 + 4] = 1;
        assert!(allowed(&settings, false, entity as u32, 1, "Engine").is_ok());
        assert!(allowed(&settings, false, entity as u32, 2, "Engine").is_err());
        assert!(allowed(&settings, true, entity as u32, 2, "Engine").is_ok());
        assert!(allowed(&settings, false, entity as u32, 0, "Engine").is_ok());
    }
}
