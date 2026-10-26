//! A car in a session is its type id (`Loadout::car_type`, MP-30): the FEng string hash of the car's folder name in upper
//! case, the hash the game data keys cars by. The lobby shows the folder back by hashing the folders it knows.

use blackbox_feng::fe_hash_upper;

/// The type id of the car in `folder` (a `folder:preset` name counts by its folder).
pub fn car_type(folder: &str) -> u32 {
    let folder = folder.split(':').next().unwrap_or(folder);
    fe_hash_upper(folder)
}

/// The name of the folder among `known` whose type id is `car_type` (a mod car's own name, else the folder), or the id in hex
/// when none is.
pub fn car_name<'a>(car_type: u32, known: impl IntoIterator<Item = &'a str>) -> String {
    if car_type == 0 {
        return "-".into();
    }
    let found = known.into_iter().find(|folder| self::car_type(folder) == car_type);
    found.map_or_else(|| format!("car {car_type:08X}"), crate::frontend::car_title::title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_and_its_presets_share_a_type_and_read_back() {
        assert_eq!(car_type("BMWM3GTR"), car_type("bmwm3gtr:M3GTRCAREERSTART"));
        assert_ne!(car_type("BMWM3GTR"), car_type("SL65"));
        let known = ["SL65", "BMWM3GTR"];
        assert_eq!(car_name(car_type("BMWM3GTR"), known), "BMWM3GTR");
        assert_eq!(car_name(0x1234, known), "car 00001234");
        assert_eq!(car_name(0, known), "-");
    }
}
