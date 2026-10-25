//! Car ids and display names from the engine's object names (`Porsche_911CarreraS_2012`).

/// Longest car folder the mod car table takes: the type record keeps `CARS\<folder>\GEOMETRY.BIN` in 32 bytes.
pub const MAX_ID: usize = 12;

/// A name split into its make and the rest, for the brand field and the display name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarName {
    pub make: String,
    pub display: String,
    /// The upper-case, digit-and-letter id before it is made unique.
    pub id: String,
}

/// Names for a car whose object name is `object_name`, or for the car numbered `fallback` when it has none.
pub fn car_name(object_name: Option<&str>, fallback: u32) -> CarName {
    let Some(name) = object_name else {
        return CarName {
            make: String::new(),
            display: format!("2012 vehicle {fallback}"),
            id: format!("V12_{fallback}"),
        };
    };
    let parts: Vec<&str> = name.split('_').collect();
    let year = parts.last().filter(|part| part.len() == 4 && part.bytes().all(|byte| byte.is_ascii_digit()));
    let model: Vec<&str> = parts[1..parts.len() - usize::from(year.is_some())].to_vec();
    let make = spaced(parts[0]);
    let mut display = make.clone();
    for part in &model {
        display.push(' ');
        display.push_str(&spaced(part));
    }
    if let Some(year) = year {
        display.push(' ');
        display.push_str(year);
    }
    CarName { id: id_of(parts[0], &model, year.copied()), make, display }
}

/// `Make3 + model + yy`, cut to [`MAX_ID`] by shortening the model.
fn id_of(make: &str, model: &[&str], year: Option<&str>) -> String {
    let alnum = |text: &str| text.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_ascii_uppercase();
    let make: String = alnum(make).chars().take(3).collect();
    let year: String = year.map(|y| y[2..].to_owned()).unwrap_or_default();
    let model: String = alnum(&model.join(""));
    let room = MAX_ID.saturating_sub(make.len() + year.len());
    format!("{make}{}{year}", model.chars().take(room).collect::<String>())
}

/// Insert spaces at word boundaries: `AstonMartin` to `Aston Martin`, `911CarreraS` to `911 Carrera S`.
fn spaced(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    for (at, &c) in chars.iter().enumerate() {
        if at > 0 {
            let prev = chars[at - 1];
            let next = chars.get(at + 1).copied();
            let lower_to_upper = prev.is_ascii_lowercase() && c.is_ascii_uppercase();
            let digit_to_letter =
                prev.is_ascii_digit() && c.is_ascii_uppercase() && next.is_some_and(|n| n.is_ascii_lowercase());
            let acronym_end =
                prev.is_ascii_uppercase() && c.is_ascii_uppercase() && next.is_some_and(|n| n.is_ascii_lowercase());
            let mc_prefix = at == 2 && chars[0] == 'M' && chars[1] == 'c';
            if (lower_to_upper && !mc_prefix) || digit_to_letter || acronym_end {
                out.push(' ');
            }
        }
        out.push(c);
    }
    out
}

/// Make `id` unique among `taken` by replacing its last characters with a counter.
pub fn unique_id(id: &str, taken: &[String]) -> String {
    if !taken.iter().any(|other| other == id) {
        return id.to_owned();
    }
    for n in 2.. {
        let suffix = n.to_string();
        let keep = id.len().min(MAX_ID - suffix.len());
        let candidate = format!("{}{suffix}", &id[..keep]);
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!("the counter never runs out")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_split_into_words() {
        let name = car_name(Some("Porsche_911CarreraS_2012"), 1);
        assert_eq!(name.display, "Porsche 911 Carrera S 2012");
        assert_eq!(name.make, "Porsche");
        assert_eq!(name.id, "POR911CARR12");
        assert_eq!(car_name(Some("AstonMartin_V12_Vantage"), 1).display, "Aston Martin V12 Vantage");
        assert_eq!(car_name(Some("BMW_1M_2012"), 1).display, "BMW 1M 2012");
        assert_eq!(car_name(Some("McLaren_F1LM_1996"), 1).display, "McLaren F1LM 1996");
    }

    #[test]
    fn ids_fit_the_limit_and_stay_unique() {
        let long = car_name(Some("Pontiac_FirebirdTransAmSpecialEdition_1977"), 1);
        assert_eq!(long.id.len(), MAX_ID);
        let taken = vec![long.id.clone()];
        let other = unique_id(&long.id, &taken);
        assert_ne!(other, long.id);
        assert!(other.len() <= MAX_ID);
    }

    #[test]
    fn a_car_without_a_name_gets_a_numbered_one() {
        let name = car_name(None, 122682);
        assert_eq!(name.id, "V12_122682");
        assert_eq!(name.display, "2012 vehicle 122682");
    }
}
