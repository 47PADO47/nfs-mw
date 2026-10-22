//! The front end: the original FEng menu packages, the game flow around them (boot movies, the title screen, the
//! main menu, free roam, the pause menu) and the screens' own logic. The packages are run by `blackbox-feng` and
//! drawn by the shared presenter in `ui`. Spec: `docs/specs/frontend-menus.md`.

mod alias_menu;
mod browse;
mod camera_move;
mod camera_pose;
mod camera_rig;
mod career_screens;
// The challenge series screen (`docs/specs/challenge-series.md`); the world records its wins.
pub(crate) mod challenge_series;
mod confirm;
mod controls_layout;
mod controls_screen;
mod credits;
pub mod dump;
mod factory;
mod flow;
pub(crate) mod graphics_options;
mod icon_items;
mod icon_menu;
mod icon_window;
mod ids;
// The ticket after a bust (`docs/specs/busted-sequence.md`); the flow opens it (`flow.rs`).
pub(crate) mod custom_race;
mod footer;
mod infractions;
pub(crate) mod input_options;
mod jukebox;
mod loading;
mod loading_tips;
mod logic;
mod look;
mod mod_options;
mod name_entry;
mod options;
mod photo;
mod plugin;
mod plugin_keys;
mod pointer;
pub(crate) mod post_options;
mod post_race;
mod prompt_layout;
mod prompts;
mod pursuit_results;
mod quick_play;
mod quick_race;
mod renderer_options;
mod rewards;
mod room;
mod scene;
mod screens;
mod script;
mod scroller;
mod splash;
mod stage;
mod widget_layout;
mod widget_menu;
// The shared option widgets. The advanced controls screen that uses them is not wired to its package yet.
#[allow(dead_code)]
mod widgets;
// The world map: opened from free roam with M, drawn by the devtools pass (`docs/specs/world-map.md`).
pub(crate) mod world_map;

#[cfg(test)]
mod bust_resume_tests;
#[cfg(test)]
mod capability_tests;
#[cfg(test)]
mod disabled_rows_tests;
#[cfg(test)]
mod exhaust_flames_tests;
#[cfg(test)]
mod flow_movie_tests;
#[cfg(test)]
mod hud_layout_tests;
#[cfg(test)]
mod menu_rows_tests;
#[cfg(test)]
mod speed_units_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_style_tests;
#[cfg(test)]
mod upscale_tests;
#[cfg(test)]
mod vehicle_effects_tests;

pub use browse::Browse;
pub use camera_rig::CameraRig;
pub use flow::{Frontend, Start};
pub use logic::{Args, Category};
pub use look::Look;
pub use plugin::{FrontendPlugin, map_pointer_on_picture};
pub use scene::MenuScene;
#[cfg(test)]
pub(crate) use scene::{Pausable, PauseFlag};
pub use script::UiScript;
