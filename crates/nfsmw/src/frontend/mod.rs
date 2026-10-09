//! The front end: the original FEng menu packages, the game flow around them (boot movies, the title screen, the
//! main menu, free roam, the pause menu) and the screens' own logic. The packages are run by `blackbox-feng` and
//! drawn by the shared presenter in `ui`. Spec: `docs/specs/frontend-menus.md`.

pub mod dump;
mod factory;
mod flow;
mod icon_menu;
mod ids;
pub(crate) mod input_options;
mod logic;
mod options;
mod plugin;
mod prompt_layout;
mod prompts;
mod scene;
mod screens;
mod script;
mod scroller;
mod splash;
mod widget_menu;

#[cfg(test)]
mod disabled_rows_tests;
#[cfg(test)]
mod exhaust_flames_tests;
#[cfg(test)]
mod hud_layout_tests;
#[cfg(test)]
mod radio_hud_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod vehicle_effects_tests;

pub use flow::Start;
pub use logic::{Args, Category};
pub use plugin::FrontendPlugin;
pub use scene::MenuScene;
pub use script::UiScript;
