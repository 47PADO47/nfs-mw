//! Finding the CustomHUD plugin in the game folder and the skin it would draw. Spec:
//! `docs/specs/custom-hud-gauges.md` section 11.

use blackbox_gauge_skin::{CarSkins, General};
use game_install::GameDir;

/// The plugin and its settings file. The skin is used only when both are there, so a player without the mod keeps the stock
/// gauges.
pub(super) const PLUGIN: &str = "scripts/CustomHud.asi";
pub(super) const SETTINGS: &str = "scripts/CustomHud.ini";

/// Read access to the game folder: paths are relative to it, case-insensitive, `/` or `\` as separators.
pub(super) trait Files {
    fn exists(&self, rel: &str) -> bool;
    fn read(&self, rel: &str) -> Option<Vec<u8>>;
}

impl Files for GameDir {
    fn exists(&self, rel: &str) -> bool {
        GameDir::exists(self, rel)
    }

    fn read(&self, rel: &str) -> Option<Vec<u8>> {
        GameDir::read(self, rel).ok()
    }
}

/// Whether the install has the plugin and its settings file.
pub(super) fn plugin_present(files: &dyn Files) -> bool {
    files.exists(PLUGIN) && files.exists(SETTINGS)
}

/// The skin chosen for a car: the plugin's settings, the folder the pictures are in and the ini's own path.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SkinSource {
    pub general: General,
    /// The folder of the skin's pictures, relative to the game folder, with no slash at either end.
    pub dir: String,
    /// The skin ini's path, relative to the game folder.
    pub ini: String,
}

/// The skin to draw for the car in folder `car`, or `None` for the stock gauges: no plugin, no skin ini, or `CustomCarHUDs = 2`
/// and the car has no skin of its own.
pub(super) fn choose(files: &dyn Files, car: &str) -> Option<SkinSource> {
    if !plugin_present(files) {
        return None;
    }
    let general = General::parse(&String::from_utf8_lossy(&files.read(SETTINGS)?));
    let own_dir = format!("CARS/{car}/CustomHUD");
    let own_ini = format!("{own_dir}/hud.ini");
    let has_own = general.car_skins != CarSkins::Off && !car.is_empty() && files.exists(&own_ini);
    if has_own {
        return Some(SkinSource { general, dir: own_dir, ini: own_ini });
    }
    if general.car_skins == CarSkins::OwnOnly {
        return None;
    }
    let dir = match general.hud_path.is_empty() {
        true => "scripts".to_owned(),
        false => format!("scripts/{}", general.hud_path),
    };
    let ini = format!("{dir}/{}", general.ini_name);
    files.exists(&ini).then_some(SkinSource { general, dir, ini })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Files in memory, keyed by lower-case path with `/` separators, as the game folder layer matches them.
    #[derive(Default)]
    struct Memory(HashMap<String, Vec<u8>>);

    impl Memory {
        fn key(rel: &str) -> String {
            rel.replace('\\', "/").to_ascii_lowercase()
        }

        fn with(mut self, rel: &str, text: &str) -> Self {
            self.0.insert(Self::key(rel), text.as_bytes().to_vec());
            self
        }
    }

    impl Files for Memory {
        fn exists(&self, rel: &str) -> bool {
            self.0.contains_key(&Self::key(rel))
        }

        fn read(&self, rel: &str) -> Option<Vec<u8>> {
            self.0.get(&Self::key(rel)).cloned()
        }
    }

    const SETTINGS_TEXT: &str = "[GENERAL]\nHUDpath = CustomHUD\\NFS15\\\nIniName = hud.ini\nCustomCarHUDs = 0\n";
    const SKIN_TEXT: &str = "[TACHOMETER_GAUGE]\nEnabled = 1\n";

    fn installed() -> Memory {
        Memory::default()
            .with("scripts/CustomHud.asi", "binary")
            .with("scripts/CustomHud.ini", SETTINGS_TEXT)
            .with("scripts/CustomHUD/NFS15/hud.ini", SKIN_TEXT)
    }

    #[test]
    fn without_the_plugin_there_is_no_skin() {
        let files = Memory::default()
            .with("scripts/CustomHud.ini", SETTINGS_TEXT)
            .with("scripts/CustomHUD/NFS15/hud.ini", SKIN_TEXT);
        assert!(!plugin_present(&files), "a stray ini does nothing");
        assert_eq!(choose(&files, "CORVETTE"), None);
        let only_plugin = Memory::default().with("scripts/CustomHud.asi", "binary");
        assert!(!plugin_present(&only_plugin));
    }

    #[test]
    fn the_general_skin_comes_from_the_settings_file() {
        let source = choose(&installed(), "CORVETTE").expect("a skin");
        assert_eq!(source.dir, "scripts/CustomHUD/NFS15");
        assert_eq!(source.ini, "scripts/CustomHUD/NFS15/hud.ini");
        assert_eq!(source.general.car_skins, CarSkins::Off);
    }

    #[test]
    fn a_missing_skin_ini_is_no_skin() {
        let files = Memory::default().with("scripts/CustomHud.asi", "b").with("scripts/CustomHud.ini", SETTINGS_TEXT);
        assert_eq!(choose(&files, "CORVETTE"), None);
    }

    #[test]
    fn a_car_with_its_own_skin_uses_it_when_the_settings_allow() {
        let settings = SETTINGS_TEXT.replace("CustomCarHUDs = 0", "CustomCarHUDs = 1");
        let files =
            installed().with("scripts/CustomHud.ini", &settings).with("CARS/CORVETTE/CustomHUD/hud.ini", SKIN_TEXT);
        let own = choose(&files, "CORVETTE").expect("the car's skin");
        assert_eq!(own.dir, "CARS/CORVETTE/CustomHUD");
        assert_eq!(own.ini, "CARS/CORVETTE/CustomHUD/hud.ini");
        let other = choose(&files, "PORSCHE").expect("the general skin");
        assert_eq!(other.dir, "scripts/CustomHUD/NFS15");
        // With the mode off the car's own skin is ignored.
        let off = installed().with("CARS/CORVETTE/CustomHUD/hud.ini", SKIN_TEXT);
        assert_eq!(choose(&off, "CORVETTE").expect("general").dir, "scripts/CustomHUD/NFS15");
    }

    #[test]
    fn mode_two_leaves_cars_without_a_skin_to_the_stock_gauges() {
        let settings = SETTINGS_TEXT.replace("CustomCarHUDs = 0", "CustomCarHUDs = 2");
        let files =
            installed().with("scripts/CustomHud.ini", &settings).with("CARS/CORVETTE/CustomHUD/hud.ini", SKIN_TEXT);
        assert!(choose(&files, "CORVETTE").is_some());
        assert_eq!(choose(&files, "PORSCHE"), None);
        assert_eq!(choose(&files, ""), None);
    }

    /// A folder in the temp directory that is removed when the test ends.
    struct Temp(std::path::PathBuf);

    impl Temp {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("nfsmw-asi-skin-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a temp folder");
            Temp(path)
        }

        fn write(&self, rel: &str, text: &str) {
            let path = self.0.join(rel);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("folders");
            std::fs::write(path, text).expect("a file");
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_game_folder_layer_finds_the_plugin_whatever_the_case() {
        let temp = Temp::new("layer");
        temp.write("Scripts/CustomHUD.ASI", "binary");
        temp.write("scripts/customhud.ini", SETTINGS_TEXT);
        temp.write("scripts/CustomHUD/NFS15/hud.ini", SKIN_TEXT);
        let dir = GameDir::open(&temp.0).expect("a folder");
        assert!(plugin_present(&dir));
        let source = choose(&dir, "CORVETTE").expect("a skin through the game folder");
        assert_eq!(source.ini, "scripts/CustomHUD/NFS15/hud.ini");
        assert!(Files::read(&dir, &source.ini).is_some());
    }

    #[test]
    fn an_empty_game_folder_has_no_plugin() {
        let temp = Temp::new("empty");
        let dir = GameDir::open(&temp.0).expect("a folder");
        assert!(!plugin_present(&dir));
        assert_eq!(choose(&dir, "CORVETTE"), None);
    }
}
