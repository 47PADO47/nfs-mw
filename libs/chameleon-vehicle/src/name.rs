use chameleon_bundle::{Bundle, ResourceKind};

/// Biggest main block of a Genesys object that is read for names, in bytes.
const MAX_NAME_OBJECT: usize = 4096;

/// Words that start behaviour and audio object names, not car names.
const NOT_A_CAR: [&str; 9] =
    ["Race", "Blacklist", "Traffic", "Chassis", "Front", "Rear", "Wheel", "Players", "LocalPlayer"];

/// The car's name as the engine's own objects spell it (`Porsche_911CarreraS_2012`), read from the vehicle's behaviour
/// objects: they are named `<car>_SteeringWheel_Behaviour`, `<car>_DamageBehaviour` and the like. Returns `None` when
/// no object carries a name with a model year, or with several parts.
pub fn vehicle_name(bundle: &Bundle) -> Option<String> {
    let kind = ResourceKind::GenesysObject.id();
    let mut votes: Vec<(String, usize)> = Vec::new();
    for (index, entry) in bundle.entries().iter().enumerate() {
        // The names sit in the small behaviour objects; the big ones are audio and physics graphs.
        if entry.kind != kind || entry.block_size(0) > MAX_NAME_OBJECT {
            continue;
        }
        let Ok(resource) = bundle.resource(index) else { continue };
        for word in words(&resource.blocks[0]) {
            let Some(name) = car_name(&word) else { continue };
            match votes.iter_mut().find(|(known, _)| *known == name) {
                Some((_, count)) => *count += 1,
                None => votes.push((name, 1)),
            }
        }
    }
    votes.into_iter().max_by_key(|(name, count)| (*count, name.len())).map(|(name, _)| name)
}

/// Tokens that end a car's name inside an object name (`AstonMartin_V12_Vantage_Steering`).
const SUFFIXES: [&str; 6] = ["Steering", "Spoiler", "Damage", "Behaviour", "Passby", "Headlights"];

fn words(data: &[u8]) -> Vec<String> {
    let is_word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    data.split(|&byte| !is_word(byte))
        .filter(|word| word.len() >= 8 && word[0].is_ascii_uppercase())
        .map(|word| String::from_utf8_lossy(word).into_owned())
        .collect()
}

/// `Make_Model_Year` out of a word like `Make_Model_Year_SteeringWheel_Behaviour`: the parts up to and including the
/// four-digit year.
fn car_name(word: &str) -> Option<String> {
    if NOT_A_CAR.iter().any(|prefix| word.starts_with(prefix)) {
        return None;
    }
    let parts: Vec<&str> = word.split('_').collect();
    let year = parts.iter().position(|part| part.len() == 4 && part.bytes().all(|b| b.is_ascii_digit()));
    if let Some(year) = year.filter(|&year| year >= 2) {
        return Some(parts[..=year].join("_"));
    }
    let end = parts.iter().position(|part| SUFFIXES.iter().any(|suffix| part.starts_with(suffix)))?;
    (end >= 2).then(|| parts[..end].join("_"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_behaviour_suffixes_after_the_year() {
        assert_eq!(
            car_name("Porsche_911CarreraS_2012_SteeringWheel_Behaviour").as_deref(),
            Some("Porsche_911CarreraS_2012")
        );
        assert_eq!(car_name("Lamborghini_Gallardo_2011").as_deref(), Some("Lamborghini_Gallardo_2011"));
    }

    #[test]
    fn rejects_other_objects() {
        assert_eq!(car_name("RaceCarEntities_Thing_2012"), None);
        assert_eq!(car_name("ChassisRigidBody"), None);
        assert_eq!(car_name("Bentley_SteeringWheel_Behaviour"), None);
        assert_eq!(car_name("AstonMartin_V12_Vantage_Steering").as_deref(), Some("AstonMartin_V12_Vantage"));
    }
}
