//! The skin: the values a theme hands to its components, looked up by name.
//!
//! A base component reads its look from the skin (`colour("panel.background")`, `font("body")`), so one
//! component renders differently under different themes with no new code. Keys are strings because mods
//! add their own; an unknown key returns `None` and the component picks its own default.

use std::collections::BTreeMap;

/// Colours, font keys, texture keys and metrics, keyed by name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Skin {
    colours: BTreeMap<String, [u8; 4]>,
    fonts: BTreeMap<String, u32>,
    textures: BTreeMap<String, u32>,
    metrics: BTreeMap<String, f32>,
}

impl Skin {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a colour `[r, g, b, a]`. Builder style: returns the skin.
    pub fn with_colour(mut self, key: impl Into<String>, rgba: [u8; 4]) -> Self {
        self.colours.insert(key.into(), rgba);
        self
    }

    /// Sets a font key (the resource handle the presenter resolves).
    pub fn with_font(mut self, key: impl Into<String>, font: u32) -> Self {
        self.fonts.insert(key.into(), font);
        self
    }

    /// Sets a texture key (the resource handle the presenter resolves).
    pub fn with_texture(mut self, key: impl Into<String>, texture: u32) -> Self {
        self.textures.insert(key.into(), texture);
        self
    }

    /// Sets a number a component uses as a size or spacing, in FEng units.
    pub fn with_metric(mut self, key: impl Into<String>, value: f32) -> Self {
        self.metrics.insert(key.into(), value);
        self
    }

    /// Adds every key of `other` to this skin. A key both skins set takes `other`'s value. Builder style.
    pub fn merged(mut self, other: Skin) -> Self {
        self.colours.extend(other.colours);
        self.fonts.extend(other.fonts);
        self.textures.extend(other.textures);
        self.metrics.extend(other.metrics);
        self
    }

    pub fn colour(&self, key: &str) -> Option<[u8; 4]> {
        self.colours.get(key).copied()
    }

    pub fn font(&self, key: &str) -> Option<u32> {
        self.fonts.get(key).copied()
    }

    pub fn texture(&self, key: &str) -> Option<u32> {
        self.textures.get(key).copied()
    }

    pub fn metric(&self, key: &str) -> Option<f32> {
        self.metrics.get(key).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::Skin;

    #[test]
    fn each_kind_round_trips_by_key() {
        let skin = Skin::new()
            .with_colour("text", [255, 255, 255, 255])
            .with_font("body", 7)
            .with_texture("bar", 9)
            .with_metric("gap", 4.5);
        assert_eq!(skin.colour("text"), Some([255, 255, 255, 255]));
        assert_eq!(skin.font("body"), Some(7));
        assert_eq!(skin.texture("bar"), Some(9));
        assert_eq!(skin.metric("gap"), Some(4.5));
    }

    #[test]
    fn unknown_key_is_none_and_kinds_do_not_mix() {
        let skin = Skin::new().with_colour("text", [0, 0, 0, 255]);
        assert_eq!(skin.colour("missing"), None);
        assert_eq!(skin.font("text"), None);
    }

    #[test]
    fn merged_adds_the_other_keys_and_the_other_value_wins() {
        let a = Skin::new().with_colour("a", [1, 1, 1, 1]).with_colour("x", [2, 2, 2, 2]);
        let b = Skin::new().with_colour("x", [3, 3, 3, 3]).with_font("f", 5);
        let merged = a.merged(b);
        assert_eq!(merged.colour("a"), Some([1, 1, 1, 1]));
        assert_eq!(merged.colour("x"), Some([3, 3, 3, 3]));
        assert_eq!(merged.font("f"), Some(5));
    }

    #[test]
    fn later_set_replaces_earlier() {
        let skin = Skin::new().with_metric("gap", 1.0).with_metric("gap", 2.0);
        assert_eq!(skin.metric("gap"), Some(2.0));
    }
}
