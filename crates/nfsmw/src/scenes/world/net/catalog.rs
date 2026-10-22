//! Car identities on the wire. A [`nfsmw_sim::PlayerJoin`] names its car by a 32-bit type id, the hash of the car's
//! folder name; an [`nfsmw_sim::EntityView`] names a car's model by a 16-bit type number, its index in the install's
//! sorted car folders. Both sides of a session check that they run the same data (`protocol.md`, `data_id`), so the
//! index means the same car on every machine.

/// The type number of a model that is not in the car folders.
pub const UNKNOWN_TYPE: u16 = u16::MAX;

/// The type id of the car in folder `folder`: the bChunk name hash of its upper-case name.
pub fn car_type_id(folder: &str) -> u32 {
    blackbox_hash::bstring_hash(&folder.to_ascii_uppercase())
}

/// The car folders of an install, sorted by name, for type ids and type numbers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CarCatalog {
    folders: Vec<String>,
}

impl CarCatalog {
    /// The catalogue of `folders`, in any order and case; a name that comes twice counts once.
    pub fn new(folders: impl IntoIterator<Item = String>) -> Self {
        let mut folders: Vec<String> = folders.into_iter().map(|name| name.to_ascii_uppercase()).collect();
        folders.sort();
        folders.dedup();
        Self { folders }
    }

    /// The folder of the car with type id `car_type`, if the install has it.
    pub fn folder(&self, car_type: u32) -> Option<&str> {
        self.folders.iter().find(|folder| car_type_id(folder) == car_type).map(String::as_str)
    }

    /// The type number of the car model in `folder` (any case), or [`UNKNOWN_TYPE`].
    pub fn type_number(&self, folder: &str) -> u16 {
        let wanted = folder.to_ascii_uppercase();
        let index = self.folders.binary_search(&wanted).ok();
        index.and_then(|index| u16::try_from(index).ok()).filter(|&n| n != UNKNOWN_TYPE).unwrap_or(UNKNOWN_TYPE)
    }

    pub fn len(&self) -> usize {
        self.folders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.folders.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> CarCatalog {
        CarCatalog::new(["copmidsize", "BMWM3GTR", "ELISE", "bmwm3gtr"].map(String::from))
    }

    #[test]
    fn folders_are_sorted_upper_case_and_unique() {
        let catalog = catalog();
        assert_eq!(catalog.len(), 3);
        assert_eq!([catalog.type_number("BMWM3GTR"), catalog.type_number("copmidsize")], [0, 1]);
        assert_eq!(catalog.type_number("elise"), 2);
        assert_eq!(catalog.type_number("NOSUCHCAR"), UNKNOWN_TYPE);
    }

    #[test]
    fn a_type_id_finds_its_folder_in_any_case() {
        let catalog = catalog();
        assert_eq!(car_type_id("elise"), car_type_id("ELISE"));
        assert_eq!(catalog.folder(car_type_id("Elise")), Some("ELISE"));
        assert_eq!(catalog.folder(car_type_id("NOSUCHCAR")), None);
        assert_ne!(car_type_id("ELISE"), car_type_id("BMWM3GTR"));
    }
}
