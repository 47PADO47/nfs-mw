//! The ids of the traffic cars: `<make and model>_TR`, at most [`crate::names::MAX_ID`] characters (the car table keeps the
//! geometry path in 32 bytes), so a traffic car never takes the id of a player car and its folder says what it is.

use chameleon_vehicle::TrafficName;

use crate::names::MAX_ID;

/// The suffix every traffic car's id ends with.
pub const SUFFIX: &str = "_TR";

/// The id of the traffic car `name`: the first three letters of its make and as much of its model as fits, in capitals and
/// digits, then `_TR`. Numbered in place of its last letters when `taken` already holds it.
pub fn id_of(name: &TrafficName, taken: &[String]) -> String {
    let alnum = |text: &str| text.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_ascii_uppercase();
    let make: String = alnum(&name.make).chars().take(3).collect();
    let stem: String = format!("{make}{}", alnum(&name.model)).chars().take(MAX_ID - SUFFIX.len()).collect();
    let first = format!("{stem}{SUFFIX}");
    if !taken.contains(&first) {
        return first;
    }
    for n in 2.. {
        let number = n.to_string();
        let keep = stem.len().min(MAX_ID - SUFFIX.len() - number.len());
        let candidate = format!("{}{number}{SUFFIX}", &stem[..keep]);
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!("the counter never runs out")
}

/// The name the garage and the devtools show: the display name of the car's facts, or the make and model in words.
pub fn display_of(name: &TrafficName, known: Option<&str>) -> String {
    let text = match known {
        Some(display) => display.to_owned(),
        None => format!("{} {}", name.make, name.model.replace('_', " ")),
    };
    format!("{text} (traffic)")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(make: &str, model: &str) -> TrafficName {
        TrafficName { make: make.to_owned(), model: model.to_owned() }
    }

    #[test]
    fn an_id_is_the_make_and_the_model_with_the_traffic_suffix() {
        assert_eq!(id_of(&name("Audi", "A3"), &[]), "AUDA3_TR");
        assert_eq!(id_of(&name("Nissan", "Versa"), &[]), "NISVERSA_TR");
        assert_eq!(id_of(&name("GMchevrolet", "Cobalt"), &[]), "GMCCOBALT_TR");
    }

    #[test]
    fn an_id_never_passes_the_limit_of_the_car_table() {
        let long = id_of(&name("Chevrolet", "ExpressVan"), &[]);
        assert_eq!(long, "CHEEXPRES_TR");
        assert!(long.len() <= MAX_ID);
        assert!(id_of(&name("Dodge", "GrandCaravan"), &[]).len() <= MAX_ID);
    }

    #[test]
    fn a_repeated_id_is_numbered_before_the_suffix() {
        let taken = vec!["CHEEXPRES_TR".to_owned()];
        let second = id_of(&name("Chevrolet", "ExpressVan"), &taken);
        assert_eq!(second, "CHEEXPRE2_TR");
        let taken = vec![taken[0].clone(), second];
        assert_eq!(id_of(&name("Chevrolet", "ExpressVan"), &taken), "CHEEXPRE3_TR");
    }

    #[test]
    fn the_display_name_says_it_is_traffic() {
        assert_eq!(display_of(&name("Audi", "A3"), Some("Audi A3")), "Audi A3 (traffic)");
        assert_eq!(display_of(&name("Volvo", "V_70"), None), "Volvo V 70 (traffic)");
    }
}
