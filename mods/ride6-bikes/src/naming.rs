//! The names of an imported bike: its car id (the folder under `CARS/` and the manifest's file name), its garage name and its
//! brand, all derived from the vehicle id (`KTM_450-SMR-2022`) so that a bike has the same names on every run.
//!
//! The vehicle id of RIDE 6 is `<Brand>_<Model words joined by ->-<Year>` (`MV-Agusta_Brutale-1090-RR-2016`); the brand has
//! no underscore, so the text before the first `_` is the brand, and the rest is the model and the year. The game's rules
//! for a car id (`docs/modding/api-content.md`): 1 to 32 letters, digits, `_` or `-`, upper case in the folder. The id is also the
//! name of the car's texture pack, which the pack header limits to 27 bytes, so an imported bike's id has at most 27.
//!
//! **The rule** ([`car_id`], [`assign`]):
//!
//! 1. The id is the vehicle id in upper case, a character that is not an ASCII letter, digit, `_` or `-` becoming `_`.
//! 2. An id of at most 27 characters is the car id as it stands (`HONDA_CBR600RR-2018`).
//! 3. A longer one keeps its first 20 characters (trailing `-` or `_` trimmed), then `-` and six upper-case hex digits of the
//!    32-bit FNV-1a hash of the vehicle id (`HONDA_CBR-1000RR-R-F-4A9C1E`): at most 27 characters, the same on every run, and
//!    nothing else about the other bikes can change it.
//! 4. Two bikes must not share an id. [`assign`] gives every vehicle of a set its id in the order of the sorted vehicle ids:
//!    when the id is taken, the hashed form is tried with the hash of `<vehicle id>#1`, then `#2`, and so on. The set of ids
//!    is therefore a function of the set of vehicles, and adding a vehicle changes another's id only if it clashes with it.
//! 5. A bike the repository already shipped under another folder keeps it ([`LEGACY`]).

use std::collections::HashSet;

/// The longest car id an imported bike gets: the game accepts 32, but the id names the texture pack, whose name the
/// pack header limits to 27 bytes.
pub const MAX_ID_LEN: usize = 27;
/// Characters of the readable head of a hashed id: the head, `-` and six hex digits fill [`MAX_ID_LEN`].
const HEAD_LEN: usize = MAX_ID_LEN - 7;
/// Vehicles whose folder was chosen before the rule existed; their car id is kept.
pub const LEGACY: [(&str, &str); 1] = [("KTM_450-SMR-2022", "KTM450")];

/// 32-bit FNV-1a.
fn fnv1a(text: &str) -> u32 {
    text.bytes().fold(0x811C_9DC5, |hash, byte| (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193))
}

/// The vehicle id upper-cased, with every character a car id may not have turned into `_`.
fn sanitized(vehicle: &str) -> String {
    vehicle
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c.to_ascii_uppercase() } else { '_' })
        .collect()
}

/// The hashed form of the id: a readable head of the sanitized id and six hex digits of the hash of the vehicle id (with
/// `#<salt>` after it for a salt above 0).
fn hashed(vehicle: &str, salt: u32) -> String {
    let source = match salt {
        0 => vehicle.to_owned(),
        n => format!("{vehicle}#{n}"),
    };
    let text = sanitized(vehicle);
    let head: String = text.chars().take(HEAD_LEN).collect();
    format!("{}-{:06X}", head.trim_end_matches(['-', '_']), fnv1a(&source) & 0x00FF_FFFF)
}

/// The car id of a vehicle on its own: rules 1 to 3 (rule 5 and the clash rule need the set, see [`assign`]).
pub fn car_id(vehicle: &str) -> String {
    let text = sanitized(vehicle);
    match text.len() <= MAX_ID_LEN {
        true => text,
        false => hashed(vehicle, 0),
    }
}

/// The car id of a vehicle imported on its own: its legacy folder if it has one, else [`car_id`]. A set of bikes should use
/// [`assign`], which also settles clashes.
pub fn default_id(vehicle: &str) -> String {
    LEGACY
        .iter()
        .find(|(known, _)| *known == vehicle)
        .map_or_else(|| car_id(vehicle), |(_, folder)| (*folder).to_owned())
}

/// The car id of every vehicle of `vehicles`, in the order given: unique, valid, stable (rules 1 to 5).
pub fn assign(vehicles: &[String]) -> Vec<String> {
    let mut sorted: Vec<&String> = vehicles.iter().collect();
    sorted.sort();
    sorted.dedup();
    let mut taken: HashSet<String> = HashSet::new();
    let mut chosen: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
    // A legacy folder is reserved first, so that no other bike can take it.
    for vehicle in &sorted {
        let Some((_, folder)) = LEGACY.iter().find(|(known, _)| known == vehicle) else { continue };
        taken.insert((*folder).to_owned());
        chosen.insert(vehicle, (*folder).to_owned());
    }
    for vehicle in sorted {
        if chosen.contains_key(vehicle.as_str()) {
            continue;
        }
        let mut id = car_id(vehicle);
        let mut salt = 0;
        while taken.contains(&id) {
            salt += 1;
            id = hashed(vehicle, salt);
        }
        taken.insert(id.clone());
        chosen.insert(vehicle, id);
    }
    vehicles.iter().map(|vehicle| chosen[vehicle.as_str()].clone()).collect()
}

/// The brand of a vehicle id: the text before the first `_`, its `-` as spaces (`MV-Agusta` is `MV Agusta`).
pub fn manufacturer(vehicle: &str) -> String {
    vehicle.split('_').next().unwrap_or("").replace('-', " ")
}

/// The garage name: brand, model and year, the separators of the id as spaces (`KTM_450-SMR-2022` is `KTM 450 SMR 2022`).
pub fn display_name(vehicle: &str) -> String {
    vehicle.replace(['_', '-'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid(id: &str) -> bool {
        (1..=MAX_ID_LEN).contains(&id.len())
            && id.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '-')
    }

    #[test]
    fn a_short_vehicle_id_is_its_upper_case() {
        assert_eq!(car_id("Honda_CBR600RR-2018"), "HONDA_CBR600RR-2018");
        assert_eq!(car_id("MV-Agusta_F4-750-1998"), "MV-AGUSTA_F4-750-1998");
        assert_eq!(car_id("A b.c"), "A_B_C");
    }

    #[test]
    fn a_long_vehicle_id_keeps_a_head_and_a_hash() {
        let vehicle = "Honda_CBR-1000RR-R-Fireblade-SP-RM-Hutchinson-2022";
        let id = car_id(vehicle);
        assert!(valid(&id), "{id}");
        assert!(id.starts_with("HONDA_CBR-1000RR-R-F-"), "{id}");
        assert_eq!(id.len(), MAX_ID_LEN);
        assert_eq!(id, car_id(vehicle), "stable");
        assert_ne!(id, car_id("Honda_CBR-1000RR-R-Fireblade-SP-RM-Hutchinson-2023"));
    }

    #[test]
    fn a_trimmed_head_never_ends_in_a_separator() {
        // The 20th character is a `-`: the head ends before it.
        let vehicle = "Aaaaaaaaaaaaaaaaaaa-Bbbbbbbbbbbbbbbbbbbbb";
        let id = car_id(vehicle);
        assert!(valid(&id));
        assert!(id.starts_with("AAAAAAAAAAAAAAAAAAA-"));
        assert_eq!(id.matches("--").count(), 0, "{id}");
    }

    #[test]
    fn the_known_bike_keeps_its_folder() {
        let all = ["Yamaha_YZF-R6-2019".to_owned(), "KTM_450-SMR-2022".to_owned()];
        assert_eq!(assign(&all), ["YAMAHA_YZF-R6-2019", "KTM450"]);
    }

    #[test]
    fn a_clash_is_resolved_by_the_order_of_the_vehicle_ids_not_by_the_order_given() {
        // `A.B` and `A_B` both sanitize to `A_B`.
        let forward = assign(&["A.B".to_owned(), "A_B".to_owned()]);
        let backward = assign(&["A_B".to_owned(), "A.B".to_owned()]);
        assert_eq!(forward[0], backward[1]);
        assert_eq!(forward[1], backward[0]);
        assert_ne!(forward[0], forward[1]);
        assert!(forward.iter().all(|id| valid(id)));
        // `A.B` sorts first and keeps the plain form.
        assert_eq!(forward[0], "A_B");
        assert!(forward[1].starts_with("A_B-"));
    }

    #[test]
    fn a_legacy_folder_is_not_given_to_another_bike() {
        // A vehicle whose plain id is the legacy folder gets another one.
        let all = ["KTM450".to_owned(), "KTM_450-SMR-2022".to_owned()];
        let ids = assign(&all);
        assert_eq!(ids[1], "KTM450");
        assert_ne!(ids[0], "KTM450");
        assert!(valid(&ids[0]));
    }

    #[test]
    fn names_come_from_the_vehicle_id() {
        assert_eq!(display_name("KTM_450-SMR-2022"), "KTM 450 SMR 2022");
        assert_eq!(manufacturer("KTM_450-SMR-2022"), "KTM");
        assert_eq!(manufacturer("MV-Agusta_Brutale-1090-RR-2016"), "MV Agusta");
        assert_eq!(display_name("Moto-Guzzi_V7-2021"), "Moto Guzzi V7 2021");
    }
}
