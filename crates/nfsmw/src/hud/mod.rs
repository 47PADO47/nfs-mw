//! The in-game HUD: the original HUD package (`HUD_SingleRace.fng`) run by `blackbox-feng`, driven by a plain
//! [`HudState`] and drawn by the shared presenter ([`crate::ui::present::blackbox`]).
//!
//! Game code only knows [`HudState`]: a scene returns one from `Scene::hud_state`. Nothing outside this module
//! names FEng or the presenter, so the presentation can change (docs/decisions/0002-ui-presentation.md).

mod bind;
#[cfg(test)]
mod bind_tests;
mod elements;
#[cfg(test)]
mod install_tests;
mod minimap;
#[cfg(test)]
mod minimap_tests;
mod plugin;
mod radio;
mod skin;
mod state;
mod trax;
mod viewport;
#[cfg(test)]
mod viewport_tests;

pub use plugin::HudPlugin;
pub use radio::{RadioHud, show as show_radio};
pub use state::{HudState, MapPosition};
