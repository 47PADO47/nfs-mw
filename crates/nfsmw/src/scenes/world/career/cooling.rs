//! Paint cooling in the garage: fitting a visual part on the career car multiplies the car's heat by the part's
//! `fecooling` factor (`docs/specs/career-cooling.md`). The garage applies a whole selection at once, so the parts that
//! differ from the applied selection are the parts fitted.

use nfsmw_career::economy::cooling::{CoolingPart, CoolingTable, SHOP_EXTRA};
use nfsmw_data::car::customization::Category;

use crate::garage::Selection;
use crate::scenes::world::WorldScene;

/// The cooling kind of a garage part category.
pub(super) fn part_of(category: Category) -> CoolingPart {
    match category {
        Category::Body => CoolingPart::BodyKit,
        Category::Spoiler => CoolingPart::Spoiler,
        Category::Hood => CoolingPart::Hood,
        Category::Roof => CoolingPart::RoofScoop,
        Category::Wheels => CoolingPart::Rim,
    }
}

/// The parts `new` fits that `old` did not have: each changed model slot, and the paint when the colour changed. A
/// different car, or no car applied yet, fits nothing (a car swap is not a part purchase).
pub(super) fn fitted_parts(old: &Selection, new: &Selection) -> Vec<CoolingPart> {
    if old.car.is_empty() || old.car != new.car {
        return Vec::new();
    }
    let mut fitted: Vec<CoolingPart> = Category::ALL
        .into_iter()
        .filter(|category| old.parts.parts[*category as usize] != new.parts.parts[*category as usize])
        .map(part_of)
        .collect();
    if old.color != new.color {
        fitted.push(CoolingPart::Paint);
    }
    fitted
}

/// The table of the install, read once. `None` when the attribute database lacks the class.
pub(super) fn read_table(dir: &game_install::GameDir) -> Option<CoolingTable> {
    let db = nfsmw_data::shared::attributes(dir).ok()?;
    let table = nfsmw_data::cooling::fe_cooling(&db)?;
    Some(CoolingTable::new([
        table.body_kit,
        table.decal,
        table.hood,
        table.numbers,
        table.paint,
        table.rim,
        table.rim_paint,
        table.roof_scoop,
        table.spoiler,
        table.vinyl,
        table.window_tint,
    ]))
}

impl WorldScene {
    /// The garage is about to commit `selection`: in career mode the parts it fits cool the current car.
    pub(in crate::scenes::world) fn career_garage_cooling(&mut self, selection: &Selection) {
        if !self.career.handle.is_active() {
            return;
        }
        let fitted = fitted_parts(&self.garage.applied, selection);
        if fitted.is_empty() {
            return;
        }
        let dir = &self.dir;
        let table = *self.career.cooling.get_or_insert_with(|| read_table(dir));
        let Some(table) = table else { return };
        let cooled = self.career.handle.with(|career| {
            let handle = career.settings.car_handle;
            let record = career.stable.career_for_handle_mut(handle)?;
            let before = record.heat;
            for part in &fitted {
                record.cool_for_part(&table, *part, SHOP_EXTRA);
            }
            Some((before, record.heat))
        });
        if let Some(Some((before, after))) = cooled {
            log::info!("career: fitted {fitted:?}, the car's heat went from {before:.2} to {after:.2}");
        }
    }
}

#[cfg(test)]
#[path = "cooling_tests.rs"]
mod tests;
