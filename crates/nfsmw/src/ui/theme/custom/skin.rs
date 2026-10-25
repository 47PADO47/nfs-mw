//! The Custom skin's fixed entries: the map legend's font and the map's icons, by key. The colours a theme shows its
//! panels and text in are not here: they are the theme's tokens (`blackbox_ui::Tokens`), which the registry writes into
//! the skin of the active theme, and the Custom HUD's `custom.*` tokens and fonts (`tokens.rs`), which the radio card and
//! the menus read. `ui/theme/mod.rs` hands the base skin to every component, so the keys are namespaced (`legend.*`);
//! the Original components never read them.

use blackbox_feng::resource_handle;
use blackbox_ui::Skin;

use crate::frontend::world_map::control::MAP_ICON_FILES;
use crate::ui::raster_text::RASTER_MONO_FONT_KEY;

/// The skin keys of the fixed entries.
pub(super) mod keys {
    /// The map legend's font: the upright monospace the FPS overlay uses, so the legend is not leaned like the cluster.
    pub const LEGEND_FONT: &str = "legend.font";
}

/// The Custom skin's own fixed entries: the legend's font and the map's icons. The Custom HUD's own colours and fonts
/// (`custom.*`) depend on the settings, so they are not here: `UiThemes` merges them from `tokens::skin`. (The radio
/// card's old body font, `radio.font`, went with the card's rewrite in PR 11b.)
pub fn card_skin() -> Skin {
    let mut skin = Skin::new().with_font(keys::LEGEND_FONT, RASTER_MONO_FONT_KEY);
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
    fn the_legend_reads_the_upright_monospace() {
        assert_eq!(card_skin().font(keys::LEGEND_FONT), Some(RASTER_MONO_FONT_KEY));
    }

    #[test]
    fn every_map_icon_has_its_texture() {
        let skin = card_skin();
        for file in MAP_ICON_FILES {
            assert_eq!(skin.texture(file), Some(resource_handle(file)), "{file}");
        }
    }
}
