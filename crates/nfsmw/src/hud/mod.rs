//! The in-game HUD: the original HUD package (`HUD_SingleRace.fng`) run by `blackbox-feng`, driven by a plain
//! [`HudState`] and drawn by the shared presenter ([`crate::ui::present::blackbox`]).
//!
//! Game code only knows [`HudState`]: a scene returns one from `Scene::hud_state`. Nothing outside this module
//! names FEng or the presenter, so the presentation can change (docs/decisions/0002-ui-presentation.md).

mod bind;
#[cfg(test)]
mod bind_tests;
mod blip_style;
#[cfg(test)]
mod blip_style_tests;
mod blip_styles;
mod blips;
#[cfg(test)]
mod blips_tests;
mod cop_blip;
#[cfg(test)]
mod cop_blip_tests;
mod elements;
#[cfg(test)]
mod install_tests;
mod interact_prompt;
#[cfg(test)]
mod interact_prompt_tests;
mod minimap;
#[cfg(test)]
mod minimap_tests;
mod plugin;
mod pursuit;
mod pursuit_end;
#[cfg(test)]
mod pursuit_tests;
mod radio;
mod skin;
mod speedbreaker;
mod state;
mod trax;
mod viewport;
#[cfg(test)]
mod viewport_tests;

pub use blip_style::MapBlip;
pub use blip_styles::{
    AUTO_SHOP, AUTO_SHOP_TINT, CAR_LOT, CAR_LOT_TINT, COP, HELICOPTER, RACE_CIRCUIT, RACE_DRAG, RACE_KNOCKOUT,
    RACE_SPEEDTRAP, RACE_SPRINT, RACE_TOLLBOOTH, SAFE_HOUSE, SAFE_HOUSE_TINT,
};
pub use interact_prompt::{InteractPopup, Prompt, show as show_interact_prompt};
pub use plugin::HudPlugin;
pub use pursuit::PursuitHud;
pub use pursuit_end::{PursuitEndCard, show as show_pursuit_end};
pub use radio::{RadioHud, show as show_radio};
pub use state::{HudState, MapPosition};
