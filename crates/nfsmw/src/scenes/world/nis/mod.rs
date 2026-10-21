//! The NIS cutscene player of the world. A scene's camera track plays in place of the gameplay camera while the world
//! stands still, for the scene's clock. Spec: `docs/specs/nis-player.md` and `nis-player-2.md`.
//!
//! This first version is camera-only: the scene's cars and characters come next, and `NisPlayer` keeps the scene's
//! anchor and time for them. The player is `player.rs`, its camera `camera.rs`, the console `command.rs`, and the
//! world's side `world.rs`.

mod camera;
mod command;
mod player;
mod world;

pub(super) use command::nis_command;
pub(super) use player::NisPlayer;
