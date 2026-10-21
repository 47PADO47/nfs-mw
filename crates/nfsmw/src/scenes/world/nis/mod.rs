//! The NIS cutscene player of the world. A scene's camera track plays in place of the gameplay camera while the world
//! stands still, for the scene's clock. Spec: `docs/specs/nis-player.md` and `nis-player-2.md`.
//!
//! The scene's cars are driven from their animation channels: `cars.rs` reads the channels and their poses, and
//! `car_world.rs` gives them the player's and the traffic's cars each frame. Characters come next. The scene's event
//! sequence runs in `events.rs`, and what its events do (the flash, the overlay, a phone call, an effect) in `handlers.rs`.
//! The player is `player.rs`, its camera `camera.rs`, the console `command.rs`, and the world's side `world.rs`.

mod camera;
mod car_world;
mod cars;
mod command;
mod events;
mod handlers;
mod player;
mod world;

pub(super) use command::nis_command;
pub(super) use player::NisPlayer;
