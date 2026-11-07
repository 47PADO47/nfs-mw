//! The gauge skin of the third-party CustomHUD plugin (`scripts/CustomHud.asi`): when the install has the plugin and the
//! player leaves `custom_hud_skin` on Auto, the original HUD's tachometer, speedometer, gear, nitrous, turbo and speed
//! breaker are hidden and the skin's gauges are drawn over the HUD, from the live telemetry.
//!
//! Spec: `docs/specs/custom-hud-gauges.md`; file format: `docs/formats/custom-hud-skin.md`. The ini parser, the gauge
//! geometry and the masks are the library `blackbox-gauge-skin`; this module finds the files, loads the pictures as UI
//! textures, binds the telemetry and builds the tree the presenter draws.
//!
//! - [`detect`]: the plugin's files and the skin chosen for a car.
//! - [`load`]: the skin's ini and its DDS pictures.
//! - [`render`]: a frame as UI nodes, the sector-masked pictures included.
//! - [`hide`]: hiding the stock gauges in the HUD tree.

mod detect;
mod hide;
#[cfg(test)]
mod install_tests;
mod load;
mod render;

pub(crate) use hide::hide_stock;

use bevy_ecs::prelude::Resource;
use blackbox_gauge_skin::{General, Inputs, Placement};
use blackbox_ui::UiTree;
use game_install::GameDir;

use super::state::HudState;
use crate::settings::{CustomHudSkin, Settings};
use crate::ui::UiAssets;
use crate::ui::present::{FENG_HEIGHT, Screen};
use detect::choose;
use load::Loaded;
use render::Overlay;

/// The skin in use: the one chosen for the player's car, or none.
#[derive(Resource)]
pub(super) struct AsiSkin {
    dir: GameDir,
    /// The car the overlay was opened for; `None` while the skin is not wanted.
    opened_for: Option<String>,
    overlay: Option<Overlay>,
}

impl AsiSkin {
    pub fn new(dir: GameDir) -> Self {
        Self { dir, opened_for: None, overlay: None }
    }

    /// Brings the overlay in step with what is wanted and the player's car (its folder name): opens the skin for a new car,
    /// drops it when it is not wanted. Returns whether a skin is drawn now.
    pub fn sync(&mut self, wanted: bool, car: &str, assets: &UiAssets) -> bool {
        let want = wanted.then(|| car.to_owned());
        if want != self.opened_for {
            self.overlay = want.as_deref().and_then(|car| open(&self.dir, car, assets));
            self.opened_for = want;
        }
        self.overlay.is_some()
    }

    /// The tree of this frame's gauges, or `None` without a skin.
    pub fn frame(&mut self, assets: &UiAssets, state: &HudState, screen: Screen, dt: f32) -> Option<UiTree> {
        let overlay = self.overlay.as_mut()?;
        let placement = placement(screen, overlay.general());
        Some(overlay.frame(assets, &inputs(state, dt), &placement))
    }
}

fn open(dir: &GameDir, car: &str, assets: &UiAssets) -> Option<Overlay> {
    let source = choose(dir, car)?;
    Loaded::open(dir, source, assets).map(Overlay::new)
}

/// Whether the skin is wanted this frame: the option is Auto, the HUD is the Original one, and no drag race has its own
/// package up.
pub(super) fn wanted(settings: &Settings, state: &HudState, original_hud: bool) -> bool {
    settings.custom_hud_skin == CustomHudSkin::Auto && original_hud && !state.drag
}

/// The skin's reference pixels on the presenter's canvas: the window's shape gives the canvas width.
fn placement(screen: Screen, general: &General) -> Placement {
    let aspect = match screen.width / screen.height {
        a if a.is_finite() && a > 0.0 => a,
        _ => 4.0 / 3.0,
    };
    Placement::new(FENG_HEIGHT * aspect, FENG_HEIGHT, general.scale, general.offset_x, general.offset_y)
}

/// The values a frame reads, from the HUD state. The speed is in the unit the player chose; the nitrous and the boost are
/// `None` for a car without them, which hides those gauges.
fn inputs(state: &HudState, dt: f32) -> Inputs {
    let to_unit = match state.use_mph {
        true => 2.236_936,
        false => 3.6,
    };
    Inputs {
        rpm: state.rpm,
        red_line: state.red_line,
        speed: state.speed * to_unit,
        mph: state.use_mph,
        gear: state.gear,
        nitrous: state.has_nos.then_some(state.nos.clamp(0.0, 1.0)),
        speedbreaker: state.speedbreaker.clamp(0.0, 1.0),
        boost: state.has_turbo.then_some(state.boost_psi),
        perfect_zone: state.perfect_launch_range,
        dt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_inputs_follow_the_hud_state_in_the_shown_unit() {
        let state = HudState {
            speed: 10.0,
            use_mph: false,
            rpm: 3500.0,
            red_line: 7000.0,
            gear: 2,
            has_nos: true,
            nos: 0.4,
            has_turbo: false,
            speedbreaker: 1.7,
            perfect_launch_range: true,
            ..HudState::default()
        };
        let inputs = inputs(&state, 0.016);
        assert!((inputs.speed - 36.0).abs() < 1e-3, "10 m/s is 36 km/h");
        assert_eq!((inputs.rpm, inputs.red_line, inputs.gear), (3500.0, 7000.0, 2));
        assert_eq!(inputs.nitrous, Some(0.4));
        assert_eq!(inputs.boost, None, "no forced induction, no boost gauge");
        assert_eq!(inputs.speedbreaker, 1.0, "the meter is clamped");
        assert!(inputs.perfect_zone);
        assert_eq!(inputs.dt, 0.016);
        let imperial = HudState { speed: 10.0, use_mph: true, ..HudState::default() };
        assert!((super::inputs(&imperial, 0.0).speed - 22.369_36).abs() < 1e-3);
        assert!(super::inputs(&imperial, 0.0).mph);
    }

    #[test]
    fn a_car_without_nitrous_hides_the_nitrous_gauge() {
        let state = HudState { has_nos: false, nos: 0.9, has_turbo: true, boost_psi: 7.0, ..HudState::default() };
        let inputs = inputs(&state, 0.0);
        assert_eq!(inputs.nitrous, None);
        assert_eq!(inputs.boost, Some(7.0));
    }

    #[test]
    fn the_canvas_is_as_wide_as_the_window_and_falls_back_to_four_by_three() {
        let general = General::default();
        let wide = placement(Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 }, &general);
        let broken = placement(Screen { width: 0.0, height: 0.0, pixels_per_point: 1.0 }, &general);
        let narrow = Placement::new(FENG_HEIGHT * 4.0 / 3.0, FENG_HEIGHT, 1.0, 0.0, 0.0);
        let rect = |p: &Placement| p.rect(100.0, 100.0, 0.0, 0.0);
        assert!(rect(&wide).x > rect(&narrow).x, "a wider window puts the corner further right");
        assert_eq!(rect(&broken), rect(&narrow));
    }

    #[test]
    fn the_skin_is_wanted_for_the_original_hud_outside_a_drag_race() {
        let settings = Settings::from(crate::settings::Partial::default());
        let state = HudState::default();
        assert!(wanted(&settings, &state, true));
        assert!(!wanted(&settings, &state, false), "the Custom and Remastered looks draw their own cluster");
        assert!(!wanted(&settings, &HudState { drag: true, ..HudState::default() }, true));
        let off = Settings::from(crate::settings::Partial {
            custom_hud_skin: Some(CustomHudSkin::Off),
            ..crate::settings::Partial::default()
        });
        assert!(!wanted(&off, &state, true));
    }
}
