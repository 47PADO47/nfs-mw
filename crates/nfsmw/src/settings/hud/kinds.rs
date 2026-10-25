//! The named choices of the `[custom_hud]` table: the preset, the map mode, the notification level and the palette.

/// A starting point for the HUD options. A preset fills only the options the player has not set
/// ([`HudPreset::layer`](super::HudPreset::layer)).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HudPreset {
    /// The least on screen: no advanced telemetry or free-roam extras, important notifications only, a compact map.
    Minimal,
    /// The defaults.
    #[default]
    Balanced,
    /// Everything on: advanced telemetry, free-roam extras, every notification, the centred map.
    Advanced,
}
names!(
    HudPreset,
    "minimal, balanced or advanced",
    [(Self::Minimal, "minimal"), (Self::Balanced, "balanced"), (Self::Advanced, "advanced")]
);

/// How the Custom HUD shows the road map. Its orientation is the `minimap` setting
/// ([`MinimapMode`](crate::settings::MinimapMode)), not a choice of its own.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MapMode {
    /// No map.
    Off,
    /// A small, fainter map centred on the screen, and a turn-cue chip near the top centre: an arrow for the next turn of
    /// the route and its distance.
    Compact,
    /// The translucent map centred on the screen, always on. The default.
    #[default]
    Centered,
    /// The centred map, larger.
    Expanded,
}
names!(
    MapMode,
    "off, compact, centered or expanded",
    [(Self::Off, "off"), (Self::Compact, "compact"), (Self::Centered, "centered"), (Self::Expanded, "expanded")]
);

/// Which notifications the Custom HUD shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NotificationLevel {
    /// Every notification. The default.
    #[default]
    All,
    /// Only the important ones.
    Important,
    /// None.
    Off,
}
names!(
    NotificationLevel,
    "all, important or off",
    [(Self::All, "all"), (Self::Important, "important"), (Self::Off, "off")]
);

/// The colour palette of the Custom HUD, for colour vision deficiencies. Every state also reads without colour.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HudPalette {
    /// The theme's own colours. The default.
    #[default]
    Default,
    /// Colours that stay apart for red-green deficiency.
    Deuteranopia,
    /// Colours that stay apart for blue-yellow deficiency.
    Tritanopia,
}
names!(
    HudPalette,
    "default, deuteranopia or tritanopia",
    [(Self::Default, "default"), (Self::Deuteranopia, "deuteranopia"), (Self::Tritanopia, "tritanopia")]
);
