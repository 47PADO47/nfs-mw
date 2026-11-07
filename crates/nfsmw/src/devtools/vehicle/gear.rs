//! The rider's gear as the developer tools change it: the same catalogue and the same file as the garage's Visual pages
//! (`garage/rider_gear`), so a choice made here is the one the profile keeps.

use crate::garage::rider_gear::{self, Catalogue, Choice, Entry, Part};

/// One part's list and what is worn from it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GearRow {
    pub part: Part,
    /// The entries of the part, the rider's own gear first.
    pub entries: Vec<Entry>,
    /// The position in `entries` of what is worn.
    pub worn: usize,
}

/// The gear rows of `catalogue` for what `choice` wears. A part whose list has only the rider's own gear is left out.
pub(crate) fn rows(catalogue: &Catalogue, choice: &Choice) -> Vec<GearRow> {
    Part::ALL
        .into_iter()
        .filter_map(|part| {
            let entries = catalogue.entries(part);
            if entries.len() < 2 {
                return None;
            }
            let worn = entries.iter().position(|entry| entry.id == choice.get(part)).unwrap_or(0);
            Some(GearRow { part, entries, worn })
        })
        .collect()
}

/// The rows of the install's catalogue and the profile's choice; `None` when no mod offers rider gear (or none has been read yet).
pub(crate) fn current() -> Option<Vec<GearRow>> {
    let catalogue = rider_gear::loaded()?;
    Some(rows(&catalogue, &rider_gear::committed()))
}

/// The id of the entry `word` names: `stock` (or `none`) is the rider's own gear, then an id, then a number counted from 1 as the
/// listing numbers them (1 is the rider's own gear, as in `--part rider-helmet=N`), then a part of a name when only one fits.
pub(crate) fn find(entries: &[Entry], word: &str) -> Result<String, String> {
    let lower = word.trim().to_ascii_lowercase();
    if matches!(lower.as_str(), "stock" | "none") {
        return Ok(String::new());
    }
    if let Some(entry) = entries.iter().find(|entry| !entry.id.is_empty() && entry.id.to_ascii_lowercase() == lower) {
        return Ok(entry.id.clone());
    }
    if let Ok(number) = lower.parse::<usize>() {
        return number
            .checked_sub(1)
            .and_then(|at| entries.get(at))
            .map(|entry| entry.id.clone())
            .ok_or_else(|| format!("the list has the numbers 1 to {}", entries.len()));
    }
    let found: Vec<&Entry> = entries.iter().filter(|entry| entry.label.to_ascii_lowercase().contains(&lower)).collect();
    match found.as_slice() {
        [one] => Ok(one.id.clone()),
        [] => Err(format!("nothing is called {word:?} (gear list shows the list)")),
        many => {
            let shown: Vec<&str> = many.iter().take(6).map(|entry| entry.label.as_str()).collect();
            Err(format!("{word:?} fits several: {}", shown.join(", ")))
        }
    }
}

/// The listing: each part with its numbered entries, the worn one starred.
pub(crate) fn text(rows: &[GearRow]) -> String {
    let mut lines = Vec::new();
    for row in rows {
        let entries: Vec<String> = row
            .entries
            .iter()
            .enumerate()
            .map(|(at, entry)| {
                let mark = match at == row.worn {
                    true => "*",
                    false => "",
                };
                format!("{}: {}{mark}", at + 1, entry.label)
            })
            .collect();
        lines.push(format!("{} ({}): {}", row.part.label(), row.part.key(), entries.join(", ")));
    }
    lines.push("`gear set <part> <number|id|name|stock>`, `gear stock`".to_owned());
    lines.join("\n")
}

/// Puts `word` on the part's rider: kept in the profile like the garage's choice, and worn at once.
pub(crate) fn set(part: Part, word: &str) -> Result<String, String> {
    let catalogue = rider_gear::loaded().ok_or_else(no_gear)?;
    let entries = catalogue.entries(part);
    let id = find(&entries, word)?;
    rider_gear::commit(part, &id).map_err(|error| format!("{error:#}"))?;
    let label = entries.iter().find(|entry| entry.id == id).map_or("Stock", |entry| entry.label.as_str());
    Ok(format!("{}: {label}{}", part.label(), disabled_note()))
}

/// Every part back to the rider's own gear.
pub(crate) fn stock() -> Result<String, String> {
    rider_gear::loaded().ok_or_else(no_gear)?;
    for part in Part::ALL {
        rider_gear::commit(part, "").map_err(|error| format!("{error:#}"))?;
    }
    Ok(format!("the rider wears his own gear{}", disabled_note()))
}

/// The text of `gear list`.
pub(crate) fn list() -> Result<String, String> {
    let rows = current().ok_or_else(no_gear)?;
    Ok(text(&rows))
}

fn no_gear() -> String {
    "no rider gear: no mod offers RIDER/GEAR/gear.json, or no bike has been shown yet in this session".to_owned()
}

/// A reminder when the setting that turns the gear off is off: the choice is kept, and not shown.
fn disabled_note() -> &'static str {
    match rider_gear::enabled() {
        true => "",
        false => " (the rider_gear setting is off: he keeps his own gear until it is on)",
    }
}

#[cfg(test)]
#[path = "gear_tests.rs"]
mod tests;
