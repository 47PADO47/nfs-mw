//! `view-car --part paint=12 --part vinyl=30`: the car with parts of the auto shop picked by number, to look at a car's
//! paint and liveries without starting the world. The numbers are the ones the `parts` console command lists.
//! `--part rider-helmet=N` (also `rider-body`, `rider-suit`, `rider-gloves`, `rider-boots`; `N` a number counted from 1 in the garage's
//! list, where 1 is the rider's own gear, or an id of the gear catalogue, or `stock`) dresses the rider of a bike in the gear a mod offers
//! (`docs/specs/ride6-rider-gear.md`). `--part plate=TEXT` puts a text on the licence plate of a car that has one (a mod car with `[customization.plate]`), and
//! `--part plate-design=ID` a design behind its glyphs (the id of the car's `designs` index; `none` is the plain plate).

use anyhow::{Context, Result, bail};
use game_install::GameDir;
use nfsmw_data::car::customization::{Catalog, Category, Selection};

use crate::garage::rider_gear::{self, Choice, Entry, Part};

/// The category a `--part` key names.
fn category(key: &str) -> Option<Category> {
    Category::from_key(key)
}

/// The plate design id a `--part plate-design=` value names: `none` (or nothing) is the plain plate, whose id is empty.
fn design_id(value: &str) -> String {
    match value.eq_ignore_ascii_case("none") {
        true => String::new(),
        false => value.to_owned(),
    }
}

/// The id of the entry a `--part rider-<part>=` value names: `stock` (or `none`) is the rider's own gear, whose id is empty; else an
/// id of the list, else a number counted from 1 in the list.
fn entry_id(entries: &[Entry], value: &str) -> Option<String> {
    if value.eq_ignore_ascii_case("stock") || value.eq_ignore_ascii_case("none") {
        return Some(String::new());
    }
    if let Some(entry) = entries.iter().find(|entry| entry.id == value) {
        return Some(entry.id.clone());
    }
    value
        .parse::<usize>()
        .ok()
        .and_then(|number| number.checked_sub(1))
        .and_then(|at| entries.get(at))
        .map(|entry| entry.id.clone())
}

/// Puts the gear the `--part rider-...` options ask for on show; the parts they leave out are the rider's own.
fn rider(dir: &GameDir, wanted: &[&String]) -> Result<()> {
    if wanted.is_empty() {
        return Ok(());
    }
    rider_gear::scan(dir);
    let catalogue =
        rider_gear::loaded().context("--part rider-...: no mod offers rider gear (RIDER/GEAR/gear.json)")?;
    let mut choice = Choice::default();
    for part in wanted {
        let (key, value) = part.split_once('=').with_context(|| format!("--part {part}: use rider-part=number"))?;
        let gear = key
            .strip_prefix("rider-")
            .and_then(Part::of_key)
            .with_context(|| format!("--part {part}: the rider's parts are body, helmet, suit, gloves and boots"))?;
        let id = entry_id(&catalogue.entries(gear), value)
            .with_context(|| format!("--part {part}: no such {}", gear.key()))?;
        choice.set(gear, &id);
    }
    rider_gear::set_preview(Some(choice));
    Ok(())
}

/// The selection `parts` ask for on the car in `folder`: each is `category=number`, with the number counted from 1 as
/// the `parts` console command lists them.
pub(super) fn selection(dir: &GameDir, folder: &str, parts: &[String]) -> Result<Selection> {
    let (gear, parts): (Vec<&String>, Vec<&String>) = parts.iter().partition(|part| part.starts_with("rider-"));
    rider(dir, &gear)?;
    let mut selection = Selection::default();
    if parts.is_empty() {
        return Ok(selection);
    }
    let mut catalog = Catalog::load(dir, folder).with_context(|| format!("listing the parts of {folder}"))?;
    crate::garage::listing::arrange(&mut catalog, &crate::i18n::game);
    for part in parts {
        let (key, number) = part.split_once('=').with_context(|| format!("--part {part}: use category=number"))?;
        if key == "plate" {
            // The text itself, not a number: the plate keeps what its sheet can draw.
            selection.plate = Some(number.to_owned());
            continue;
        }
        if key == "plate-design" {
            selection.plate_design = Some(design_id(number));
            continue;
        }
        let Some(category) = category(key) else {
            bail!(
                "--part {part}: the category is body, spoiler, hood, roof, rims, paint, tint, rimpaint, vinyl, plate or plate-design"
            );
        };
        let choice = number
            .parse::<usize>()
            .ok()
            .and_then(|number| number.checked_sub(1))
            .and_then(|at| catalog.choices[category as usize].get(at))
            .with_context(|| format!("--part {part}: {folder} has no such {key}"))?;
        selection.parts[category as usize] = Some(choice.hash);
    }
    Ok(selection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_are_the_names_of_the_parts_command() {
        assert_eq!(category("vinyl"), Some(Category::Vinyl));
        assert_eq!(category("rimpaint"), Some(Category::RimPaint));
        assert_eq!(category("wings"), None);
    }

    fn entries() -> Vec<Entry> {
        ["", "A", "B"].iter().map(|id| Entry { id: (*id).to_owned(), label: (*id).to_owned() }).collect()
    }

    #[test]
    fn a_rider_gear_value_is_stock_an_id_or_a_number_counted_from_one() {
        assert_eq!(entry_id(&entries(), "stock"), Some(String::new()));
        assert_eq!(entry_id(&entries(), "NONE"), Some(String::new()));
        assert_eq!(entry_id(&entries(), "B"), Some("B".to_owned()));
        assert_eq!(entry_id(&entries(), "1"), Some(String::new()), "the first is the rider's own gear");
        assert_eq!(entry_id(&entries(), "3"), Some("B".to_owned()));
        assert_eq!(entry_id(&entries(), "4"), None);
        assert_eq!(entry_id(&entries(), "0"), None);
        assert_eq!(entry_id(&entries(), "C"), None);
    }

    #[test]
    fn the_plain_plate_is_the_design_none() {
        assert_eq!(design_id("none"), "");
        assert_eq!(design_id("NONE"), "");
        assert_eq!(design_id("1863272"), "1863272");
    }
}
