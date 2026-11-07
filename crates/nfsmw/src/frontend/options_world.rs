//! The Free Roam World row of the Gameplay options (`docs/specs/second-world.md`): Rockport, then every second world a loaded
//! mod provides. The choice is the `world` setting; a plain Free Roam start plays in it. The row is listed only while a mod
//! provides a world.

use super::Data;
use super::steps::cycle;
use crate::i18n::t;
use crate::modding::content::worlds;
use crate::settings::world_id::CITY;
use crate::settings::{Partial, Settings, WorldId};

/// The ids on offer: the city first, then the worlds of the mods.
fn choices() -> Vec<(String, String)> {
    let city = (CITY.to_owned(), t!("options-world-city"));
    std::iter::once(city).chain(worlds::list().into_iter().map(|w| (w.id().to_owned(), w.name().to_owned()))).collect()
}

/// Whether any mod provides a world, so the row is worth listing.
pub(super) fn offered() -> bool {
    !worlds::list().is_empty()
}

/// The chosen world's name; the city when the setting is unset or names a world that is not installed.
pub(super) fn data(s: &Settings) -> Data {
    let id = s.world.map_or_else(|| CITY.to_owned(), |id| id.to_string());
    let name = choices().into_iter().find(|(known, _)| known.eq_ignore_ascii_case(&id)).map(|(_, name)| name);
    Data::Text(name.unwrap_or_else(|| t!("options-world-city")))
}

/// Moves to the next (`forward`) or previous world and records it in `changed`. Returns whether the world changed.
pub(super) fn step(s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
    let choices = choices();
    if choices.len() < 2 {
        return false;
    }
    let current = s.world.map_or_else(|| CITY.to_owned(), |id| id.to_string());
    let at = choices.iter().position(|(id, _)| id.eq_ignore_ascii_case(&current)).unwrap_or(0);
    let Ok(next) = choices[cycle(at, choices.len(), forward)].0.parse::<WorldId>() else { return false };
    s.world = Some(next);
    changed.world = Some(next);
    true
}
