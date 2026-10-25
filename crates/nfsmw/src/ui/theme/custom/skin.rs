//! The Custom skin's fixed entries: the map's icons, by key. The colours and fonts a theme shows its panels and text in
//! are not here: they are the theme's tokens (`blackbox_ui::Tokens`), which the registry writes into the skin of the
//! active theme, and the Custom HUD's `custom.*` tokens and fonts (`tokens.rs`). `ui/theme/mod.rs` hands the base skin
//! to every component; the Original components never read these keys.

use blackbox_feng::resource_handle;
use blackbox_ui::Skin;

use crate::frontend::world_map::control::MAP_ICON_FILES;

/// The Custom skin's own fixed entries: the map's icons. The Custom HUD's own colours and fonts (`custom.*`) depend on
/// the settings, so they are not here: `UiThemes` merges them from `tokens::skin`.
pub fn card_skin() -> Skin {
    let mut skin = Skin::new();
    // The map's blip icons, keyed by their file name, so a map draws an icon by the file `icon_name` returns.
    for file in MAP_ICON_FILES {
        skin = skin.with_texture(file, resource_handle(file));
    }
    skin
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_map_icon_has_its_texture() {
        let skin = card_skin();
        for file in MAP_ICON_FILES {
            assert_eq!(skin.texture(file), Some(resource_handle(file)), "{file}");
        }
    }
}
