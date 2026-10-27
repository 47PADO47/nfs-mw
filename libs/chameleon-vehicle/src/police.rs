//! Which bundles are police vehicles. The 2012 game keeps no role flag on a vehicle; what marks a police vehicle is in its
//! bundle: a second lamp object for the siren coronas (named `<Car>_Cop_Coronas`), a `LightBarHinge` part in its damage
//! object, and, for the SWAT van, its own name. Evidence and the cars: `docs/formats/chameleon-police.md`.

use chameleon_bundle::{Bundle, ResourceKind};

/// Biggest main block of a Genesys object that is read, in bytes.
const MAX_OBJECT: usize = 65_536;

/// What a bundle says about the vehicle being a police vehicle.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PoliceMarks {
    /// The name in the siren corona object (`Dodge_Charger_SRT8_Cop_Coronas` gives `Dodge_Charger_SRT8_Cop`).
    pub siren_name: Option<String>,
    /// The damage object lists a light bar part (`LightBarHinge`).
    pub light_bar: bool,
    /// An object is named after the SWAT van (`NFS_SwatVan_2012_Coronas`).
    pub swat: bool,
}

impl PoliceMarks {
    /// Whether any mark says the vehicle is a police vehicle.
    pub fn is_police(&self) -> bool {
        self.siren_name.is_some() || self.light_bar || self.swat
    }

    fn add(&mut self, word: &str) {
        if word.contains("LightBarHinge") {
            self.light_bar = true;
        }
        let Some(name) = word.strip_suffix("_Coronas") else { return };
        let tokens: Vec<&str> = name.split('_').collect();
        if tokens.iter().any(|token| token.eq_ignore_ascii_case("cop")) && self.siren_name.is_none() {
            self.siren_name = Some(name.to_owned());
        }
        if tokens.iter().any(|token| token.to_ascii_lowercase().starts_with("swat")) {
            self.swat = true;
        }
    }
}

/// The police marks of a vehicle bundle.
pub fn police_marks(bundle: &Bundle) -> PoliceMarks {
    let kind = ResourceKind::GenesysObject.id();
    let mut marks = PoliceMarks::default();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != kind || entry.block_size(0) > MAX_OBJECT {
            continue;
        }
        let Ok(resource) = bundle.resource(index) else { continue };
        marks_of_block(&resource.blocks[0], &mut marks);
    }
    marks
}

/// Add the marks of one object's main block.
fn marks_of_block(data: &[u8], marks: &mut PoliceMarks) {
    let is_word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    for word in data.split(|&byte| !is_word(byte)).filter(|word| word.len() >= 8) {
        marks.add(&String::from_utf8_lossy(word));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marks(text: &str) -> PoliceMarks {
        let mut marks = PoliceMarks::default();
        marks_of_block(text.as_bytes(), &mut marks);
        marks
    }

    #[test]
    fn a_siren_corona_object_names_the_police_car() {
        let found = marks("\0\0Dodge_Charger_SRT8_Cop_Coronas\0\0");
        assert_eq!(found.siren_name.as_deref(), Some("Dodge_Charger_SRT8_Cop"));
        assert!(found.is_police());
        assert_eq!(marks("Ford_Crown_Victoria_COP_Coronas").siren_name.as_deref(), Some("Ford_Crown_Victoria_COP"));
    }

    #[test]
    fn a_light_bar_part_or_the_swat_van_marks_a_police_car() {
        assert!(marks("\u{1}?LightBarHinge\0").light_bar);
        assert!(marks("NFS_SwatVan_2012_Coronas").swat);
    }

    #[test]
    fn an_ordinary_car_has_no_mark() {
        let found = marks("Porsche_911_Targa4S_Coronas Race Cop Passbys Racer_And_Cop_Passbys");
        assert!(!found.is_police(), "a sound group named for cops is not a siren");
        assert!(!marks("Ford_Mustang_Shelby_GT500_Coronas").is_police());
    }
}
