//! The rewards the career hands out (`docs/specs/story-flow-2.md` section 4.5, F-25): the rule crate's [`grant`] applied to the
//! career, then the pink slips it reports added to the stable. The rewards of a gauntlet's last race use it
//! (`scenes/world/career/events.rs`), and so does the markers screen, which grants each card the player reveals.
//!
//! The rule crate reports a pink slip as [`RewardEffect::AddCar`] and does not add it, because the stable needs the car
//! lot's entry of the preset (its handle and keys). The car lot is read from [`Career::data`] by the preset's name.

use nfsmw_career::blacklist::{Reward, RewardEffect, grant};
use nfsmw_career::carlot::{LotCar, add_preset_car};
use nfsmw_career::economy::markers::MarkerInventory;
use nfsmw_career::stable::Stable;

use super::{Career, CareerData};

/// Grants `rewards` to `career`, in order. With `deferred` every routable reward goes to the popup queue instead of being
/// applied. Each pink slip the rule crate reports is added to the stable at once. The marker inventory is read back
/// afterwards, because the profile is saved from the markers. Returns one effect per reward, in order.
pub fn grant_rewards(career: &mut Career, rewards: &[Reward], deferred: bool) -> Vec<RewardEffect> {
    let effects = grant(rewards, deferred, &mut career.settings, &mut career.popups, |preset| {
        owns_preset(&career.stable, &career.data, preset)
    });
    read_markers(career);
    for effect in &effects {
        if let RewardEffect::AddCar { preset } = effect {
            add_pink_slip(career, preset);
        }
    }
    effects
}

/// Reads the marker inventory back from the settings, so the markers and the saved table agree.
fn read_markers(career: &mut Career) {
    match MarkerInventory::from_table(&career.settings.inventory[..]) {
        Ok(markers) => career.markers = markers,
        Err(e) => log::warn!("the marker inventory could not be read after the rewards: {e:?}"),
    }
}

/// Whether the stable holds the preset car `preset`, under the handle the car lot gives it. A name the car lot does not
/// list is not held.
fn owns_preset(stable: &Stable, data: &CareerData, preset: &str) -> bool {
    preset_car(data, preset).is_some_and(|car| stable.car_by_handle(car.handle).is_some())
}

/// The car lot's entry of the preset car `preset`, by name.
fn preset_car<'a>(data: &'a CareerData, preset: &str) -> Option<&'a LotCar> {
    data.cars.iter().find(|car| car.name.eq_ignore_ascii_case(preset))
}

/// Adds the preset car `preset` to the stable (F-25). The car lot's entry gives the handle and the keys; the first free
/// customization record is its customization record. A name the car lot does not list, a stable with no free
/// customization record, or a refused add changes nothing and is logged.
fn add_pink_slip(career: &mut Career, preset: &str) {
    let Some(car) = preset_car(&career.data, preset) else {
        return log::warn!("reward: pink slip {preset} is not in the car lot, so no car is added");
    };
    let Some(customization) = career.stable.free_customization() else {
        return log::warn!("reward: pink slip {preset} has no free customization record, so no car is added");
    };
    match add_preset_car(&mut career.stable, car, customization) {
        Ok(added) => log::info!("reward: pink slip {preset} added to the stable in slot {}", added.car_slot),
        Err(e) => log::info!("reward: pink slip {preset} is not added: {e:?}"),
    }
}

#[cfg(test)]
#[path = "rewards_tests.rs"]
mod tests;
