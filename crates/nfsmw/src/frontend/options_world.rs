//! The Free Roam World row of the Gameplay options (`docs/specs/second-world.md`): Rockport, then every second world a loaded
//! mod provides. The choice is the `world` setting; a plain Free Roam start plays in it. The row is listed only while a mod
//! provides a world.

use super::Data;
use super::steps::cycle;
use crate::i18n::t;
use crate::modding::content::worlds;
use crate::settings::world_id::CITY;
use crate::settings::{Partial, Settings, WorldId};

/// One choice of the row: the world's id and the name the menu shows.
type Choice = (String, String);

/// The choices on offer: the city first, then the worlds of the mods.
fn choices() -> Vec<Choice> {
    let city = (CITY.to_owned(), t!("options-world-city"));
    std::iter::once(city).chain(worlds::list().into_iter().map(|w| (w.id().to_owned(), w.name().to_owned()))).collect()
}

/// Whether any mod provides a world, so the row is worth listing.
pub(super) fn offered() -> bool {
    !worlds::list().is_empty()
}

/// The id the setting names, the city when it is unset.
fn current(s: &Settings) -> String {
    s.world.map_or_else(|| CITY.to_owned(), |id| id.to_string())
}

/// What the row shows: the chosen world's name; the city's when the setting names a world that is not installed.
pub(super) fn data(s: &Settings) -> Data {
    Data::Text(name_in(&choices(), &current(s)))
}

fn name_in(choices: &[Choice], id: &str) -> String {
    let found = choices.iter().find(|(known, _)| known.eq_ignore_ascii_case(id));
    found.map_or_else(|| choices[0].1.clone(), |(_, name)| name.clone())
}

/// Moves to the next (`forward`) or previous world and records it in `changed`. Returns whether the world changed.
pub(super) fn step(s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
    step_in(&choices(), s, changed, forward)
}

fn step_in(choices: &[Choice], s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
    if choices.len() < 2 {
        return false;
    }
    let current = current(s);
    let at = choices.iter().position(|(id, _)| id.eq_ignore_ascii_case(&current)).unwrap_or(0);
    let Ok(next) = choices[cycle(at, choices.len(), forward)].0.parse::<WorldId>() else { return false };
    s.world = Some(next);
    changed.world = Some(next);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> Vec<Choice> {
        vec![(CITY.to_owned(), "Rockport".to_owned()), ("okayama".to_owned(), "Okayama (RIDE 6)".to_owned())]
    }

    #[test]
    fn the_row_cycles_through_the_city_and_the_worlds_and_records_the_choice() {
        let mut s = Settings::load(Partial::default());
        let mut changed = Partial::default();
        assert!(s.world.is_none());
        assert!(step_in(&list(), &mut s, &mut changed, true));
        assert_eq!(s.world.map(|w| w.to_string()).as_deref(), Some("okayama"));
        assert_eq!(changed.world, s.world);
        assert!(step_in(&list(), &mut s, &mut changed, true), "after the last world comes the city");
        assert_eq!(s.world.map(|w| w.to_string()).as_deref(), Some(CITY));
        assert!(step_in(&list(), &mut s, &mut changed, false), "and back from the city is the last world");
        assert_eq!(s.world.map(|w| w.to_string()).as_deref(), Some("okayama"));
    }

    #[test]
    fn with_no_world_to_choose_nothing_moves_and_an_unknown_setting_reads_as_the_city() {
        let mut s = Settings::load(Partial::default());
        let mut changed = Partial::default();
        assert!(!step_in(&list()[..1], &mut s, &mut changed, true));
        assert!(changed.world.is_none());
        s.world = "gone".parse().ok();
        assert_eq!(name_in(&list(), &current(&s)), "Rockport");
        s.world = "OKAYAMA".parse().ok();
        assert_eq!(name_in(&list(), &current(&s)), "Okayama (RIDE 6)");
    }
}
