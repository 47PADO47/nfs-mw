//! The Custom HUD's transition durations, in one table (`docs/specs/custom-hud/visual-identity.md` section 7). Every
//! animation runs on the frame clock's elapsed seconds (`CustomFeed::elapsed_s`), never on frame counts.
//!
//! Motion reduction (`[custom_hud] motion_reduction`) takes the movement out and keeps the information: a slide becomes a
//! fade in place, a pulse becomes a static highlight held as long as the pulse, and mode and map changes become short
//! crossfades. The speed scale keeps its spring: it is a reading, not decoration.

use crate::settings::hud::HudSettings;

/// The transition durations, in seconds, and the distances that go with them, in HUD units at scale 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    /// Any element appearing.
    pub fade_in: f32,
    /// Any element leaving.
    pub fade_out: f32,
    /// Notifications entering, the race strip arriving: how long, and how far the element travels.
    pub slide: f32,
    pub slide_distance: f32,
    /// A one-shot pulse (nitrous on, a checkpoint confirmed, heat up). Without motion it is a highlight held this long.
    pub pulse: f32,
    /// Whether the pulse moves (grows and settles) or is a static highlight.
    pub pulse_moves: bool,
    /// The free-roam, race and pursuit layouts crossfading.
    pub mode_change: f32,
    /// The map changing mode.
    pub map_mode: f32,
    /// A map zoom step (a critically damped spring on the view radius settles in about this long).
    pub map_zoom: f32,
    /// The speed scale's spring settles in about this long.
    pub speed_spring: f32,
    /// How long the bounty delta stays before it fades.
    pub delta_hold: f32,
}

impl Motion {
    /// The full motion: the spec's table.
    pub const FULL: Motion = Motion {
        fade_in: 0.150,
        fade_out: 0.250,
        slide: 0.200,
        slide_distance: 12.0,
        pulse: 0.400,
        pulse_moves: true,
        mode_change: 0.300,
        map_mode: 0.300,
        map_zoom: 0.250,
        speed_spring: 0.060,
        delta_hold: 2.0,
    };

    /// Motion reduced: no travel, no moving pulse, short crossfades for the mode and map changes.
    pub const REDUCED: Motion = Motion {
        slide_distance: 0.0,
        pulse_moves: false,
        mode_change: 0.150,
        map_mode: 0.150,
        map_zoom: 0.150,
        ..Motion::FULL
    };

    /// The full or the reduced table.
    pub fn of(reduced: bool) -> Self {
        match reduced {
            true => Self::REDUCED,
            false => Self::FULL,
        }
    }

    /// The table `[custom_hud] motion_reduction` chooses.
    pub fn for_settings(settings: &HudSettings) -> Self {
        Self::of(settings.motion_reduction)
    }
}

impl Default for Motion {
    fn default() -> Self {
        Self::FULL
    }
}

/// How far a transition of `duration` seconds is `age` seconds after it started, 0 to 1. A transition with no length is
/// already done.
pub fn ramp(age: f32, duration: f32) -> f32 {
    if duration <= 0.0 || duration.is_nan() {
        return 1.0;
    }
    (age / duration).clamp(0.0, 1.0)
}
