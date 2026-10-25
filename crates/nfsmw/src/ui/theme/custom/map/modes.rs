//! The map's modes (`docs/specs/custom-hud-map-impl.md`): what each draws, how one turns into the next, and the keys
//! that change them.
//!
//! - **Off:** nothing.
//! - **Compact:** a small, fainter overlay round the car, and the turn-cue chip near the top centre.
//! - **Centered:** the translucent overlay round the car, in the middle of the content box.
//! - **Expanded:** a larger, more opaque overlay that shows farther, with the paths and the stretch of the route
//!   already driven.
//!
//! The overlays are 340, 500 and 710 pixels wide at 1080p (`docs/specs/custom-hud/map.md`).
//!
//! A mode change blends the old mode's [`Overlay`] into the new one's over `Motion::map_mode`: the ellipse and the
//! scale glide, and the overlay and the chip fade in or out. With motion reduction the shape and the scale take the new
//! mode at once and only the fades run, so the change is a short crossfade.
//!
//! The keys (`MapMode`, `MapExpand`, `MapZoomIn`, `MapZoomOut`, `MapOrientation`, `ToggleHud`) change the live
//! settings through [`MapKeys::apply`], which the HUD plugin calls when one is pressed (`hud/map_keys.rs`).

use crate::settings::hud::{MAP_ZOOM_RANGE, MapMode};
use crate::settings::{MinimapMode, Settings};

/// A zoom key multiplies or divides `map_zoom` by this.
pub const ZOOM_STEP: f32 = 1.25;
/// The ellipses' half-axes (horizontal, vertical), as fractions of the content box's height.
pub const COMPACT_RADII: [f32; 2] = [0.155, 0.115];
pub const CENTERED_RADII: [f32; 2] = [0.23, 0.17];
pub const EXPANDED_RADII: [f32; 2] = [0.33, 0.245];
/// The world radius, metres, the ellipse's horizontal half-axis shows at zoom 1.
pub const COMPACT_BASE_M: f32 = 200.0;
pub const CENTERED_BASE_M: f32 = 300.0;
pub const EXPANDED_BASE_M: f32 = 600.0;
/// How faint the Compact overlay is, as a share of `map_opacity`.
pub const COMPACT_OPACITY_SHARE: f32 = 0.7;
/// How far the Expanded map raises the roads' opacity over `map_opacity`, and the most it raises it to.
pub const EXPANDED_OPACITY_GAIN: f32 = 0.25;
pub const EXPANDED_OPACITY_CAP: f32 = 0.85;

/// The next mode for the `MapMode` key: off, compact, centered, expanded, then off again.
pub fn next_mode(mode: MapMode) -> MapMode {
    match mode {
        MapMode::Off => MapMode::Compact,
        MapMode::Compact => MapMode::Centered,
        MapMode::Centered => MapMode::Expanded,
        MapMode::Expanded => MapMode::Off,
    }
}

/// The `minimap` setting after the `MapOrientation` key: fixed and rotating swap, and off becomes rotating.
pub fn flip_orientation(mode: MinimapMode) -> MinimapMode {
    match mode {
        MinimapMode::Fixed | MinimapMode::Off => MinimapMode::Rotating,
        MinimapMode::Rotating => MinimapMode::Fixed,
    }
}

/// `zoom` one step in (a closer view), within the setting's range.
pub fn zoom_in(zoom: f32) -> f32 {
    (zoom * ZOOM_STEP).clamp(MAP_ZOOM_RANGE.0, MAP_ZOOM_RANGE.1)
}

/// `zoom` one step out (a wider view), within the setting's range.
pub fn zoom_out(zoom: f32) -> f32 {
    (zoom / ZOOM_STEP).clamp(MAP_ZOOM_RANGE.0, MAP_ZOOM_RANGE.1)
}

/// The road opacity of the Expanded map for `map_opacity`: raised, but never above the cap unless the setting is.
pub fn expanded_opacity(map_opacity: f32) -> f32 {
    map_opacity.max((map_opacity + EXPANDED_OPACITY_GAIN).min(EXPANDED_OPACITY_CAP))
}

/// Which of the map's keys were pressed this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapPresses {
    pub mode: bool,
    pub expand: bool,
    pub zoom_in: bool,
    pub zoom_out: bool,
    pub orientation: bool,
    pub toggle_hud: bool,
}

impl MapPresses {
    pub fn any(&self) -> bool {
        self.mode || self.expand || self.zoom_in || self.zoom_out || self.orientation || self.toggle_hud
    }
}

/// What the keys remember between presses: the mode the expanded map was opened from, to go back to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapKeys {
    opened_from: Option<MapMode>,
}

impl MapKeys {
    /// Applies this frame's presses to the live settings: the map's mode and zoom (`hud_options`), the orientation
    /// (`minimap`) and the HUD's visibility (`hud`). Nothing here is saved.
    pub fn apply(&mut self, presses: MapPresses, settings: &mut Settings) {
        if presses.mode {
            settings.hud_options.map_mode = next_mode(settings.hud_options.map_mode);
            self.opened_from = None;
        }
        if presses.expand {
            settings.hud_options.map_mode = self.expand(settings.hud_options.map_mode);
        }
        if presses.zoom_in {
            settings.hud_options.map_zoom = zoom_in(settings.hud_options.map_zoom);
        }
        if presses.zoom_out {
            settings.hud_options.map_zoom = zoom_out(settings.hud_options.map_zoom);
        }
        if presses.orientation {
            settings.minimap = flip_orientation(settings.minimap);
        }
        if presses.toggle_hud {
            settings.hud = !settings.hud;
        }
    }

    /// The mode after `MapExpand` from `mode`: the expanded map, or back from it to the mode it was opened from
    /// (Centered when it was not opened by this key).
    fn expand(&mut self, mode: MapMode) -> MapMode {
        if mode == MapMode::Expanded {
            return self.opened_from.take().unwrap_or(MapMode::Centered);
        }
        self.opened_from = Some(mode);
        MapMode::Expanded
    }
}

/// What one mode draws, as numbers that blend: the overlay's ellipse and scale, its roads' opacity, and how much of
/// the overlay, its extra detail and the turn-cue chip shows (0 or 1 for a settled mode).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Overlay {
    /// The ellipse's half-axes as fractions of the content box's height.
    pub radii: [f32; 2],
    /// The world radius, metres, of the horizontal half-axis at zoom 1.
    pub base_m: f32,
    pub road_opacity: f32,
    /// The overlay's own fade.
    pub shown: f32,
    /// The Expanded map's extra detail: the paths and the stretch of the route already driven.
    pub detail: f32,
    /// The turn-cue chip.
    pub chip: f32,
}

impl Overlay {
    /// The overlay of `mode` with the roads at `map_opacity`. Off keeps the Centered shape, hidden, so a change from it
    /// only fades.
    pub fn of(mode: MapMode, map_opacity: f32) -> Self {
        let centered = Self {
            radii: CENTERED_RADII,
            base_m: CENTERED_BASE_M,
            road_opacity: map_opacity,
            shown: 1.0,
            detail: 0.0,
            chip: 0.0,
        };
        match mode {
            MapMode::Centered => centered,
            MapMode::Expanded => Self {
                radii: EXPANDED_RADII,
                base_m: EXPANDED_BASE_M,
                road_opacity: expanded_opacity(map_opacity),
                detail: 1.0,
                ..centered
            },
            MapMode::Compact => Self {
                radii: COMPACT_RADII,
                base_m: COMPACT_BASE_M,
                road_opacity: map_opacity * COMPACT_OPACITY_SHARE,
                chip: 1.0,
                ..centered
            },
            MapMode::Off => Self { shown: 0.0, ..centered },
        }
    }

    /// The overlay `t` (0 to 1) of the way from `self` to `to`.
    pub fn blend(self, to: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let mix = |a: f32, b: f32| a + (b - a) * t;
        Self {
            radii: [mix(self.radii[0], to.radii[0]), mix(self.radii[1], to.radii[1])],
            base_m: mix(self.base_m, to.base_m),
            road_opacity: mix(self.road_opacity, to.road_opacity),
            shown: mix(self.shown, to.shown),
            detail: mix(self.detail, to.detail),
            chip: mix(self.chip, to.chip),
        }
    }
}

/// The overlay `progress` (0 to 1, linear) of the way through a change from `from` to `to`. The blend is eased. With
/// motion `reduced` the shape and the scale are `to`'s at once and only the fades blend.
pub fn transition(from: Overlay, to: Overlay, progress: f32, reduced: bool) -> Overlay {
    let blended = from.blend(to, ease(progress));
    if !reduced {
        return blended;
    }
    Overlay { radii: to.radii, base_m: to.base_m, ..blended }
}

/// Smoothstep: starts and ends gently.
pub fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
