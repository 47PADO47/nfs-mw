//! The traffic bundles of the install (`VEHICLES/VEH_<n>_TR.BNDL`): a traffic car is a car bundle with no stats, no paints,
//! no skeleton and no driver, only its body at five levels of detail, its wheels, its lamps and its collision soup. Its
//! name is the one thing the bundle says about it, in the name of its lamp object. Spec: `docs/formats/chameleon-traffic.md`.

use chameleon_bundle::{Bundle, ResourceKind};

/// Biggest main block of a Genesys object that is read for names, in bytes.
const MAX_NAME_OBJECT: usize = 4096;
/// The name of a traffic car's lamp object starts and ends with these (`Traffic_Audi_A3_Coronas`).
const PREFIX: &str = "Traffic_";
const SUFFIX: &str = "_Coronas";

/// The make and model of a traffic car as the engine spells them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrafficName {
    /// `Audi`, `GMchevrolet`.
    pub make: String,
    /// `A3`, `GrandCaravan`: the words after the make, joined by `_` as they were written (`Ram1500`, `ExpressVan`).
    pub model: String,
}

impl TrafficName {
    /// The name as the engine writes it between `Traffic_` and `_Coronas`: `Audi_A3`.
    pub fn object_name(&self) -> String {
        format!("{}_{}", self.make, self.model)
    }
}

/// The name of the traffic car in `bundle`, read from its lamp object (`Traffic_<Make>_<Model>_Coronas`). `None` for a bundle
/// that has no such object, which is every car bundle (`_HI`, `_LO`).
pub fn traffic_name(bundle: &Bundle) -> Option<TrafficName> {
    let kind = ResourceKind::GenesysObject.id();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != kind || entry.block_size(0) > MAX_NAME_OBJECT {
            continue;
        }
        let Ok(resource) = bundle.resource(index) else { continue };
        if let Some(name) = resource.blocks[0].split(|&byte| !is_word(byte)).find_map(parse) {
            return Some(name);
        }
    }
    None
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// `Traffic_Audi_A3_Coronas` to `Audi` and `A3`.
fn parse(word: &[u8]) -> Option<TrafficName> {
    let word = std::str::from_utf8(word).ok()?;
    let inner = word.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    let (make, model) = inner.split_once('_')?;
    if make.is_empty() || model.is_empty() {
        return None;
    }
    Some(TrafficName { make: make.to_owned(), model: model.to_owned() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lamp_object_name_gives_the_make_and_the_model() {
        let name = parse(b"Traffic_Audi_A3_Coronas").unwrap();
        assert_eq!((name.make.as_str(), name.model.as_str()), ("Audi", "A3"));
        assert_eq!(name.object_name(), "Audi_A3");
        let van = parse(b"Traffic_Chevrolet_ExpressVan_Coronas").unwrap();
        assert_eq!((van.make.as_str(), van.model.as_str()), ("Chevrolet", "ExpressVan"));
        let two = parse(b"Traffic_Dodge_Grand_Caravan_Coronas").unwrap();
        assert_eq!(two.model, "Grand_Caravan", "the words after the make stay together");
    }

    #[test]
    fn words_that_are_not_a_traffic_lamp_object_are_not_names() {
        assert!(parse(b"Porsche_911CarreraS_2012_Coronas").is_none());
        assert!(parse(b"Traffic_Audi_Coronas").is_none(), "a make and no model");
        assert!(parse(b"Traffic__A3_Coronas").is_none());
        assert!(parse(b"Traffic_Audi_A3_SteeringWheel").is_none());
        assert!(parse(&[0xff, 0xfe]).is_none());
    }

    #[test]
    #[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
    fn the_eleven_traffic_bundles_are_named_and_no_car_bundle_is() {
        let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
        let mut traffic = Vec::new();
        for entry in std::fs::read_dir(std::path::Path::new(&dir).join("VEHICLES")).unwrap().flatten() {
            let file = entry.file_name().to_string_lossy().to_uppercase();
            let is_traffic = file.starts_with("VEH_") && file.ends_with("_TR.BNDL");
            let is_car = file.starts_with("VEH_") && (file.ends_with("_HI.BNDL") || file.ends_with("_LO.BNDL"));
            if !is_traffic && !is_car {
                continue;
            }
            let data = std::fs::read(entry.path()).unwrap();
            let name = traffic_name(&Bundle::parse(&data).unwrap());
            assert_eq!(name.is_some(), is_traffic, "{file}: {name:?}");
            traffic.extend(name.map(|name| name.object_name()));
        }
        traffic.sort();
        assert_eq!(
            traffic,
            [
                "Audi_A3",
                "Cadillac_CTSV",
                "Chevrolet_ExpressVan",
                "Dodge_Caliber",
                "Dodge_GrandCaravan",
                "Dodge_Ram1500",
                "GMC_Topkick",
                "GMchevrolet_Cobalt",
                "Infiniti_G35",
                "Nissan_Frontier",
                "Nissan_Versa"
            ]
        );
    }
}

#[cfg(test)]
mod low_detail_tests {
    use std::path::Path;

    use chameleon_bundle::{Bundle, Pool};

    use crate::{find_spec, load};

    /// The triangles of each body level of the car bundle `file` under `VEHICLES/`.
    fn levels(dir: &Path, textures: &[u8], shaders: &[u8], file: &str) -> Vec<usize> {
        let data = std::fs::read(dir.join("VEHICLES").join(file)).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        let spec = find_spec(&bundle).unwrap();
        let mut pool = Pool::new();
        pool.add(bundle.clone());
        pool.add(Bundle::parse(textures).unwrap());
        pool.add(Bundle::parse(shaders).unwrap());
        let vehicle = load(&pool, spec).unwrap();
        vehicle.body.iter().map(|level| level.iter().map(|mesh| mesh.triangles.len()).sum()).collect()
    }

    /// The claims of `docs/formats/chameleon-traffic.md` about the `_LO` bundles, counted on the install: every `_HI` car has a
    /// `_LO` twin of 5 or 6 body levels, the twin holds the car's own coarse levels, and 92 of 93 add a mid-detail level.
    #[test]
    #[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
    fn the_low_detail_bundles_hold_a_mid_level_the_car_bundles_lack() {
        let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
        let dir = Path::new(&dir);
        let textures = std::fs::read(dir.join("VEHICLES/VEHICLETEX.BNDL")).unwrap();
        let shaders = std::fs::read(dir.join("SHADERS.BNDL")).unwrap();
        let (mut cars, mut with_mid, mut sharing_second) = (0, 0, 0);
        for entry in std::fs::read_dir(dir.join("VEHICLES")).unwrap().flatten() {
            let file = entry.file_name().to_string_lossy().to_uppercase();
            let Some(number) = file.strip_prefix("VEH_").and_then(|rest| rest.strip_suffix("_HI.BNDL")) else {
                continue;
            };
            let high = levels(dir, &textures, &shaders, &file);
            let low = levels(dir, &textures, &shaders, &format!("VEH_{number}_LO.BNDL"));
            assert!((5..=6).contains(&low.len()), "{file}: {low:?}");
            cars += 1;
            sharing_second += usize::from(high.get(1).is_some_and(|second| low.contains(second)));
            let mid = low.iter().filter(|&&t| t >= 2000 && (t as f32) <= 0.6 * high[0] as f32).min();
            with_mid += usize::from(mid.is_some());
        }
        println!(
            "{cars} cars, {with_mid} with a mid level, {sharing_second} whose second level the _LO bundle has too"
        );
        assert_eq!((cars, with_mid), (93, 92));
        assert_eq!(sharing_second, 87, "the cars whose second level the _LO bundle repeats");
    }
}
