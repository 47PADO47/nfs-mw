//! The settings file that chooses the skin (`[GENERAL]`): which folder and ini, the global scale and offset, and whether a car
//! may bring its own skin. Spec: `docs/formats/custom-hud-skin.md` section 2.

use crate::doc::Doc;

/// What `CustomCarHUDs` asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CarSkins {
    /// 0: the general skin only.
    #[default]
    Off,
    /// 1: a car's own skin when it has one, else the general skin.
    Own,
    /// 2: a car's own skin; a car without one draws no custom HUD (the stock gauges stay).
    OwnOnly,
}

#[derive(Debug, Clone, PartialEq)]
pub struct General {
    /// The skin's folder, as written in the file with its backslashes turned into `/` and no slash at either end.
    pub hud_path: String,
    /// The skin's ini file name in that folder.
    pub ini_name: String,
    /// A Windows virtual-key code that reloads the skin; 0 is off.
    pub hot_reload_key: i64,
    pub show_debug_info: bool,
    pub car_skins: CarSkins,
    /// Multiplies every size, position and offset.
    pub scale: f32,
    /// Added to every position, in reference pixels.
    pub offset_x: f32,
    pub offset_y: f32,
    /// The Carbon section's shake amount. Most Wanted has no engine shake, so it is not used there.
    pub shake_amount: f32,
}

impl Default for General {
    fn default() -> Self {
        Self {
            hud_path: String::new(),
            ini_name: "hud.ini".to_owned(),
            hot_reload_key: 0,
            show_debug_info: false,
            car_skins: CarSkins::Off,
            scale: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            shake_amount: 0.0,
        }
    }
}

/// A folder name from a skin ini: `\` and `/` both separate, and no separator at either end.
pub fn normalise_dir(text: &str) -> String {
    text.trim().replace('\\', "/").trim_matches('/').to_owned()
}

impl General {
    /// Reads the text of the settings file. A missing `Scale` is 1 **[ours]**.
    pub fn parse(text: &str) -> Self {
        let doc = Doc::parse(text);
        let car_skins = match doc.int("GENERAL", "CustomCarHUDs", 0) {
            1 => CarSkins::Own,
            2 => CarSkins::OwnOnly,
            _ => CarSkins::Off,
        };
        let ini_name = Some(doc.text("GENERAL", "IniName")).filter(|name| !name.is_empty());
        Self {
            hud_path: normalise_dir(&doc.text("GENERAL", "HUDpath")),
            ini_name: ini_name.unwrap_or_else(|| Self::default().ini_name),
            hot_reload_key: doc.int("GENERAL", "HotReloadKey", 0),
            show_debug_info: doc.flag("GENERAL", "ShowDebugInfo", false),
            car_skins,
            scale: doc.float("GENERAL", "Scale", 1.0),
            offset_x: doc.float("GENERAL", "OffsetX", 0.0),
            offset_y: doc.float("GENERAL", "OffsetY", 0.0),
            shake_amount: doc.float("CARBON", "ShakeAmount", 0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[GENERAL]\nHUDpath = CustomHUD\\NFS15\\\nIniName = hud.ini\nHotReloadKey = 0\nShowDebugInfo = 0\nCustomCarHUDs = 1 // 0 - disable, 1 - enable\n\nScale = 1.0\nOffsetX = 2.5\nOffsetY = -4\n\n[CARBON]\nShakeAmount = 500";

    #[test]
    fn the_settings_file_is_read_with_its_backslashes_normalised() {
        let general = General::parse(SAMPLE);
        assert_eq!(general.hud_path, "CustomHUD/NFS15");
        assert_eq!(general.ini_name, "hud.ini");
        assert_eq!(general.car_skins, CarSkins::Own);
        assert_eq!((general.scale, general.offset_x, general.offset_y), (1.0, 2.5, -4.0));
        assert_eq!(general.shake_amount, 500.0);
        assert!(!general.show_debug_info);
        assert_eq!(general.hot_reload_key, 0);
    }

    #[test]
    fn an_empty_file_gives_the_defaults() {
        let general = General::parse("");
        assert_eq!(general, General::default());
        assert_eq!(general.scale, 1.0);
        assert_eq!(general.ini_name, "hud.ini");
    }

    #[test]
    fn car_skin_modes_read_zero_one_and_two() {
        let mode = |value: &str| General::parse(&format!("[GENERAL]\nCustomCarHUDs = {value}\n")).car_skins;
        assert_eq!(mode("0"), CarSkins::Off);
        assert_eq!(mode("1"), CarSkins::Own);
        assert_eq!(mode("2"), CarSkins::OwnOnly);
        assert_eq!(mode("9"), CarSkins::Off);
    }

    #[test]
    fn directory_names_lose_their_edges_and_separators() {
        assert_eq!(normalise_dir("\\CustomHUD\\NFS15\\"), "CustomHUD/NFS15");
        assert_eq!(normalise_dir("a/b"), "a/b");
        assert_eq!(normalise_dir(""), "");
    }
}
