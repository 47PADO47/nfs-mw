//! The in-game HUD: the original HUD package (`HUD_SingleRace.fng`) run by `blackbox-feng`, driven by a plain
//! [`HudState`] and drawn by exactly one presenter ([`present::blackbox`]).
//!
//! Game code only knows [`HudState`]: a scene returns one from `Scene::hud_state`. Nothing outside this module
//! names FEng or the presenter, so the presentation can change (docs/decisions/0002-ui-presentation.md).

mod assets;
mod bind;
mod plugin;
pub mod present;
mod state;

pub use plugin::HudPlugin;
pub use state::HudState;
