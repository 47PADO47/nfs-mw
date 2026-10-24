//! The `[custom_hud]` options the Custom menus take (`docs/specs/custom-hud/menus.md` section 2): the text size and
//! motion reduction. The palette and high contrast reach the menus through the `custom.*` colours like the HUD's; the
//! HUD scale does not, because the menus have their own scale.
//!
//! The menus' components are built from the skin alone, so the two options travel in it as metrics
//! ([`MenuLook::skin`]), merged by `UiThemes::refresh_skin` next to the `custom.*` colours.

use blackbox_ui::Skin;

use crate::settings::hud::{HudSettings, TEXT_SIZE_RANGE};

/// The skin metric of the menus' text size, a factor (`[custom_hud] text_size`).
pub const TEXT_SIZE: &str = "custom.menu.text_size";
/// The skin metric of motion reduction: 1 when it is on, 0 when it is off (`[custom_hud] motion_reduction`).
pub const REDUCE_MOTION: &str = "custom.menu.reduce_motion";

/// What the menus take from `[custom_hud]`. The text size is kept in hundredths, so a look compares exactly and the
/// registry can tell when it changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MenuLook {
    text_size_pct: u16,
    reduce_motion: bool,
}

impl MenuLook {
    /// The look `[custom_hud]` chooses.
    pub fn of(settings: &HudSettings) -> Self {
        Self::new(settings.text_size, settings.motion_reduction)
    }

    /// The look of a text size (clamped to its range) and a motion reduction switch.
    pub fn new(text_size: f32, reduce_motion: bool) -> Self {
        let (low, high) = TEXT_SIZE_RANGE;
        let size = match text_size.is_finite() {
            true => text_size.clamp(low, high),
            false => 1.0,
        };
        Self { text_size_pct: (size * 100.0).round() as u16, reduce_motion }
    }

    /// The text size, a factor of the menus' own sizes.
    pub fn text_size(self) -> f32 {
        f32::from(self.text_size_pct) / 100.0
    }

    /// Whether the selection moves without a glide.
    pub fn reduce_motion(self) -> bool {
        self.reduce_motion
    }

    /// The two metrics, for the skin the components read.
    pub fn skin(self) -> Skin {
        let reduce = match self.reduce_motion {
            true => 1.0,
            false => 0.0,
        };
        Skin::new().with_metric(TEXT_SIZE, self.text_size()).with_metric(REDUCE_MOTION, reduce)
    }

    /// The look a skin carries. A skin without the metrics (one built before the registry merged them) reads as the
    /// defaults.
    pub fn from_skin(skin: &Skin) -> Self {
        let text_size = skin.metric(TEXT_SIZE).unwrap_or(1.0);
        let reduce = skin.metric(REDUCE_MOTION).is_some_and(|on| on > 0.5);
        Self::new(text_size, reduce)
    }
}

impl Default for MenuLook {
    fn default() -> Self {
        Self::of(&HudSettings::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_look_round_trips_through_the_skin_and_clamps_the_text_size() {
        let look = MenuLook::new(1.25, true);
        assert_eq!(MenuLook::from_skin(&look.skin()), look);
        assert!((look.text_size() - 1.25).abs() < 1e-6);
        assert!(look.reduce_motion());
        assert_eq!(MenuLook::from_skin(&Skin::new()), MenuLook::default(), "no metrics: the defaults");
        assert!((MenuLook::new(9.0, false).text_size() - TEXT_SIZE_RANGE.1).abs() < 1e-6);
        assert!((MenuLook::new(f32::NAN, false).text_size() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_look_follows_the_hud_options_but_not_the_hud_scale() {
        let settings = HudSettings { text_size: 1.4, motion_reduction: true, scale: 1.5, ..HudSettings::default() };
        let look = MenuLook::of(&settings);
        assert_eq!(look, MenuLook::new(1.4, true));
        assert_eq!(look, MenuLook::of(&HudSettings { scale: 0.75, ..settings }), "the HUD scale is not the menus'");
    }
}
