//! The car lists of the developer tools (`garage` in the world and car viewers, `pursuit cops`): every car folder of the
//! install under the category the game gives it, with the cars of mods tagged by what they are. A stock car is a racing car
//! or a police car by its car-table usage; a mod car is a player car, a police car (cop role) or both, with its unit class
//! and its wave entries (`nfsmw_data::car::mods::CopUnit::describe`).

use game_install::GameDir;
use nfsmw_data::car::CarTables;
use nfsmw_data::car::mods::{self as car_mods, ModCar};

/// What a car folder is, for the listings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A stock car of the player's lists (car-table usage 0).
    Racing,
    /// A stock police car (usage 1).
    Police,
    /// A folder that is no car type of the table, or a type of another use.
    Other,
    /// A mod car that only the player drives.
    ModPlayer,
    /// A mod car that the player drives and the cops chase with, with its unit as the devtools describe it.
    ModBoth(String),
    /// A mod car that only the cops drive, with its unit.
    ModPolice(String),
    /// A mod car that is only traffic, with its traffic unit.
    ModTraffic(String),
}

/// The kind of a mod car.
pub(crate) fn mod_kind(car: &ModCar) -> Kind {
    if let (None, false, Some(unit)) = (&car.cop, car.player, &car.traffic) {
        return Kind::ModTraffic(unit.describe());
    }
    match (&car.cop, car.player) {
        (None, _) => Kind::ModPlayer,
        (Some(unit), true) => Kind::ModBoth(unit.describe()),
        (Some(unit), false) => Kind::ModPolice(unit.describe()),
    }
}

/// The kind of the car in `folder`, from the mods' cars first, then the car table's usage (`None` table: unknown).
pub(crate) fn kind_of(folder: &str, tables: Option<&CarTables>) -> Kind {
    if let Some(car) = car_mods::published().iter().find(|car| car.folder.eq_ignore_ascii_case(folder)) {
        return mod_kind(car);
    }
    match tables.and_then(|tables| tables.car_type_for_folder(folder)).map(|car_type| car_type.usage_type) {
        Some(0) => Kind::Racing,
        Some(1) => Kind::Police,
        _ => Kind::Other,
    }
}

/// The listing of every car folder of the install, one line per category.
pub(crate) fn garage_list(dir: &GameDir) -> String {
    let tables = CarTables::load(dir).ok();
    let entries = nfsmw_data::car::list(dir).into_iter().map(|folder| {
        let kind = kind_of(&folder, tables.as_ref());
        (folder, kind)
    });
    render(entries.collect())
}

/// The lines of a listing: the folders of each kind together, the tags of the mod cars after their names.
pub(crate) fn render(entries: Vec<(String, Kind)>) -> String {
    let join = |wanted: &dyn Fn(&Kind) -> Option<String>, entries: &[(String, Kind)]| -> String {
        let items: Vec<String> =
            entries.iter().filter_map(|(folder, kind)| wanted(kind).map(|tag| format!("{folder}{tag}"))).collect();
        items.join("  ")
    };
    let plain = |want: Kind| move |kind: &Kind| (*kind == want).then(String::new);
    let tagged = |kind: &Kind| match kind {
        Kind::ModBoth(unit) => Some(format!(" [{unit}]")),
        _ => None,
    };
    let police = |kind: &Kind| match kind {
        Kind::ModPolice(unit) => Some(format!(" [{unit}]")),
        _ => None,
    };
    let traffic = |kind: &Kind| match kind {
        Kind::ModTraffic(unit) => Some(format!(" [{unit}]")),
        _ => None,
    };
    let sections: [(&str, String); 7] = [
        ("racing cars", join(&plain(Kind::Racing), &entries)),
        ("police cars", join(&plain(Kind::Police), &entries)),
        ("mod player cars", join(&plain(Kind::ModPlayer), &entries)),
        ("mod police cars (cop only)", join(&police, &entries)),
        ("mod cars that are both player and police", join(&tagged, &entries)),
        ("mod traffic cars", join(&traffic, &entries)),
        ("other folders", join(&plain(Kind::Other), &entries)),
    ];
    let lines: Vec<String> = sections
        .into_iter()
        .filter(|(_, items)| !items.is_empty())
        .map(|(title, items)| format!("{title}: {items}"))
        .collect();
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_has_its_own_line_and_mod_cops_carry_their_unit() {
        let entries = vec![
            ("BMWM3GTR".to_owned(), Kind::Racing),
            ("COPMIDSIZE".to_owned(), Kind::Police),
            ("FASTCOUPE".to_owned(), Kind::ModPlayer),
            ("SUVCOP".to_owned(), Kind::ModPolice("heavy, heat 6 x2 @80%".to_owned())),
            ("DUAL".to_owned(), Kind::ModBoth("patrol, heat 1 x2 @10%".to_owned())),
            ("HATCH_TR".to_owned(), Kind::ModTraffic("compact, weight 20, no limit, x1, every pattern".to_owned())),
        ];
        let text = render(entries);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "racing cars: BMWM3GTR");
        assert_eq!(lines[1], "police cars: COPMIDSIZE");
        assert_eq!(lines[2], "mod player cars: FASTCOUPE");
        assert_eq!(lines[3], "mod police cars (cop only): SUVCOP [heavy, heat 6 x2 @80%]");
        assert_eq!(lines[4], "mod cars that are both player and police: DUAL [patrol, heat 1 x2 @10%]");
        assert_eq!(lines[5], "mod traffic cars: HATCH_TR [compact, weight 20, no limit, x1, every pattern]");
        assert_eq!(lines.len(), 6, "a category with no car has no line");
    }

    #[test]
    fn a_stock_folder_without_a_table_is_other() {
        assert_eq!(kind_of("NOSUCHCAR", None), Kind::Other);
    }
}
