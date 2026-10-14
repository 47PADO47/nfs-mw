//! The front end: the original FEng menu packages, the game flow around them (boot movies, the title screen, the
//! main menu, free roam, the pause menu) and the screens' own logic. The packages are run by `blackbox-feng` and
//! drawn by the shared presenter in `ui`. Spec: `docs/specs/frontend-menus.md`.

mod browse;
mod confirm;
mod controls_layout;
mod controls_screen;
pub mod dump;
mod factory;
mod flow;
mod graphics_options;
mod icon_items;
mod icon_menu;
mod icon_window;
mod ids;
pub(crate) mod input_options;
mod loading;
mod logic;
mod options;
mod plugin;
mod pointer;
pub(crate) mod post_options;
mod prompt_layout;
mod prompts;
mod renderer_options;
mod room;
mod scene;
mod screens;
mod script;
mod scroller;
mod splash;
mod track_options;
mod track_select;
mod widget_layout;
mod widget_menu;
// The shared option widgets. The advanced controls screen that uses them is not wired to its package yet.
#[allow(dead_code)]
mod widgets;
// The world map: opened from free roam with M, drawn by the devtools pass (`docs/specs/world-map.md`).
pub(crate) mod world_map;

#[cfg(test)]
mod capability_tests;
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
mod upscale_tests;
#[cfg(test)]
mod vehicle_effects_tests;

pub use browse::Browse;
pub use flow::Start;
pub use logic::{Args, Category};
pub use plugin::FrontendPlugin;
pub use scene::MenuScene;
pub use script::UiScript;
