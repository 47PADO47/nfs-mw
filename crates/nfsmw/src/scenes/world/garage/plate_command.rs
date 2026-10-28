//! The `plate` and `plate-design` console commands: the licence plate of the car being driven, for a car that has one.
//!
//! They change the plate the way the F6 garage does (`garage/plate_edit.rs`): the selection of the active car is applied at
//! once, for the session only. A car without a `[customization.plate]` answers "this car has no plate"; a plate without
//! designs answers the same to `plate-design`. The garage's License Plate and Plate Design pages are what keep a plate beside
//! the profile's save (`plates.toml`).

use blackbox_gfx::RenderBackend;
use nfsmw_data::car::customization::{Catalog, PlateDesignChoice};
use nfsmw_data::car::mods::custom::Plate;

use super::WorldScene;
use crate::garage::{CatalogSnapshot, plate_edit};

/// How many designs `plate-design list` prints at once.
const LISTED: usize = 30;

/// What a `plate` line asks for.
#[derive(Debug, PartialEq, Eq)]
enum TextCommand {
    Show,
    /// The text the car wears outside the console: the garage's kept one, else the manifest's.
    Reset,
    Set(String),
}

fn text_command(args: &[&str]) -> TextCommand {
    match args {
        [] => TextCommand::Show,
        [word] if word.eq_ignore_ascii_case("reset") => TextCommand::Reset,
        _ => TextCommand::Set(args.join(" ")),
    }
}

/// What a `plate-design` line asks for.
#[derive(Debug, PartialEq, Eq)]
enum DesignCommand {
    Show,
    /// The designs, only the ones whose id or name has the text.
    List(Option<String>),
    /// The design the car wears outside the console.
    Reset,
    /// An id or a name of the list, or `none`.
    Set(String),
}

fn design_command(args: &[&str]) -> DesignCommand {
    match args {
        [] => DesignCommand::Show,
        [word] if word.eq_ignore_ascii_case("reset") => DesignCommand::Reset,
        [word] if word.eq_ignore_ascii_case("list") => DesignCommand::List(None),
        [word, filter @ ..] if word.eq_ignore_ascii_case("list") => DesignCommand::List(Some(filter.join(" "))),
        _ => DesignCommand::Set(args.join(" ")),
    }
}

/// The car being driven, its plate and its catalogue.
fn plated(scene: &mut WorldScene) -> Result<(String, &'static Plate, CatalogSnapshot), String> {
    if scene.players.is_empty() {
        return Err("get into a car first (use the drive command)".into());
    }
    let car = scene.garage.applied.car.clone();
    scene.discover_garage_parts(&car).map_err(|e| format!("{e:#}"))?;
    let catalog = scene.garage.cars.iter().find(|c| c.folder == car).and_then(|c| c.parts.clone());
    let Some(catalog) = catalog else { return Err(format!("{car} is not in the garage list")) };
    let plate = plate_edit::supported(&catalog, &car).ok_or("this car has no plate")?;
    Ok((car, plate, catalog))
}

/// The car's selection with `edit` done to its parts, applied to the car.
fn apply(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    edit: impl FnOnce(&mut nfsmw_data::car::customization::Selection),
) -> Result<(), String> {
    let mut selection = scene.garage.applied.clone();
    edit(&mut selection.parts);
    scene.apply_garage_selection(renderer, selection).map_err(|e| format!("{e:#}"))
}

/// The plate as the car wears it: its text, and its design when the plate offers designs.
fn status(car: &str, plate: &Plate, catalog: &Catalog, picked: (Option<&str>, Option<&str>)) -> String {
    let text = plate_edit::worn(plate, car, picked.0);
    let mut line = format!("{car} plate: {:?}", text.trim_end());
    if !catalog.plate_designs.is_empty() {
        line.push_str(&format!(", design: {}", plate_edit::design_name(plate, catalog, car, picked.1)));
    }
    line
}

/// `plate [TEXT|reset]`.
pub(in crate::scenes::world) fn plate_text(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    args: &[&str],
) -> Result<String, String> {
    let (car, plate, catalog) = plated(scene)?;
    let command = text_command(args);
    match command {
        TextCommand::Show => {}
        TextCommand::Reset => apply(scene, renderer, |parts| parts.plate = None)?,
        TextCommand::Set(text) => {
            let text = plate_edit::checked(plate, &text)?;
            let chosen = plate_edit::chosen(plate, &text);
            apply(scene, renderer, |parts| parts.plate = chosen)?;
        }
    }
    let parts = &scene.garage.applied.parts;
    Ok(status(&car, plate, &catalog, (parts.plate.as_deref(), parts.plate_design.as_deref())))
}

/// `plate-design [ID|none|list|reset]`.
pub(in crate::scenes::world) fn plate_design(
    scene: &mut WorldScene,
    renderer: &mut dyn RenderBackend,
    args: &[&str],
) -> Result<String, String> {
    let (car, plate, catalog) = plated(scene)?;
    if catalog.plate_designs.is_empty() {
        return Err("this car has no plate designs".into());
    }
    match design_command(args) {
        DesignCommand::Show => {}
        DesignCommand::List(filter) => return Ok(list(&catalog.plate_designs, filter.as_deref())),
        DesignCommand::Reset => apply(scene, renderer, |parts| parts.plate_design = None)?,
        DesignCommand::Set(wanted) => {
            let id = plate_edit::design_id(&catalog.plate_designs, &wanted)?;
            apply(scene, renderer, |parts| parts.plate_design = Some(id))?;
        }
    }
    let parts = &scene.garage.applied.parts;
    Ok(status(&car, plate, &catalog, (parts.plate.as_deref(), parts.plate_design.as_deref())))
}

/// The designs, one `id: name` per line (with their group), the first `LISTED` of those whose id or name has `filter`.
fn list(designs: &[PlateDesignChoice], filter: Option<&str>) -> String {
    let wanted = filter.map(str::to_ascii_lowercase);
    let matching: Vec<&PlateDesignChoice> = designs
        .iter()
        .filter(|design| {
            wanted.as_ref().is_none_or(|wanted| {
                design.id.to_ascii_lowercase().contains(wanted) || design.name.to_ascii_lowercase().contains(wanted)
            })
        })
        .collect();
    if matching.is_empty() {
        return "no plate design matches".into();
    }
    let mut lines = vec!["none: the plain plate".to_owned()];
    for design in matching.iter().take(LISTED) {
        let group = design.group.as_deref().map(|group| format!(" [{group}]")).unwrap_or_default();
        lines.push(format!("{}: {}{group}", design.id, design.name));
    }
    if matching.len() > LISTED {
        lines.push(format!("... {} more (plate-design list <part of a name> narrows it)", matching.len() - LISTED));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn designs(count: usize) -> Vec<PlateDesignChoice> {
        (1..=count)
            .map(|n| PlateDesignChoice {
                id: n.to_string(),
                name: format!("Design {n}"),
                group: (n % 2 == 0).then(|| "Even".to_owned()),
            })
            .collect()
    }

    #[test]
    fn the_text_command_has_three_forms() {
        assert_eq!(text_command(&[]), TextCommand::Show);
        assert_eq!(text_command(&["reset"]), TextCommand::Reset);
        assert_eq!(text_command(&["RESET"]), TextCommand::Reset);
        assert_eq!(text_command(&["MY", "CAR"]), TextCommand::Set("MY CAR".into()), "spaces are the blank glyph");
        assert_eq!(text_command(&["velocity"]), TextCommand::Set("velocity".into()));
    }

    #[test]
    fn the_design_command_has_five_forms() {
        assert_eq!(design_command(&[]), DesignCommand::Show);
        assert_eq!(design_command(&["list"]), DesignCommand::List(None));
        assert_eq!(design_command(&["list", "flag", "of"]), DesignCommand::List(Some("flag of".into())));
        assert_eq!(design_command(&["reset"]), DesignCommand::Reset);
        assert_eq!(design_command(&["none"]), DesignCommand::Set("none".into()));
        assert_eq!(design_command(&["1863272"]), DesignCommand::Set("1863272".into()));
        assert_eq!(design_command(&["Flag", "Of", "Nowhere"]), DesignCommand::Set("Flag Of Nowhere".into()));
    }

    #[test]
    fn the_list_is_bounded_and_can_be_narrowed() {
        let all = list(&designs(100), None);
        assert!(all.starts_with("none: the plain plate\n1: Design 1\n2: Design 2 [Even]"));
        assert_eq!(all.lines().count(), 1 + LISTED + 1, "the plain plate, the first designs, the count of the rest");
        assert!(all.contains("... 70 more"));
        let some = list(&designs(100), Some("DESIGN 10"));
        assert!(some.contains("10: Design 10 [Even]") && !some.contains("more"));
        assert_eq!(list(&designs(3), Some("nothing like it")), "no plate design matches");
    }

    #[test]
    fn the_status_names_the_text_and_only_a_plate_with_designs_names_a_design() {
        let plate = Plate {
            text: Some("NEED4SPD".into()),
            slots: 8,
            glyphs: "ABCDEFGHIJKLMNOPQRSTUVWXYZ 4".chars().collect(),
            ..Default::default()
        };
        let plain = Catalog { plate: true, ..Catalog::default() };
        assert_eq!(
            status("NO_SUCH_CAR_ANYWHERE", &plate, &plain, (None, None)),
            "NO_SUCH_CAR_ANYWHERE plate: \"NEED4SPD\""
        );
        let designed = Catalog { plate: true, plate_designs: designs(2), ..Catalog::default() };
        let plate = Plate { designs: Some("designs.toml".into()), ..plate };
        assert_eq!(
            status("NO_SUCH_CAR_ANYWHERE", &plate, &designed, (Some("FAST"), Some("2"))),
            "NO_SUCH_CAR_ANYWHERE plate: \"FAST\", design: Design 2"
        );
    }
}
