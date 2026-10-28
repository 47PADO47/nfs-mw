//! The input layer: devices -> actions.
//!
//! Game code reads [`ActionState`] and never a key code, mouse event or window event. The mapping from
//! devices (keyboard, mouse, gamepad) to [`Action`]s is the [`Bindings`] resource, so controllers and
//! rebinding need no change in the game code. The layer sits on `bevy_input`, which Bevy's own renderer
//! would also use, so it carries over to a full Bevy move.

mod action;
mod bindings;
pub mod commands;
#[cfg(test)]
mod config_tests;
mod describe;
mod device_settings;
#[cfg(test)]
mod headlights_tests;
pub mod hud_labels;
#[cfg(test)]
mod interact_tests;
mod presentation;
#[cfg(test)]
mod presentation_tests;
mod remap;
#[cfg(test)]
mod rider_weight_tests;
mod slots;
mod snapshot;
mod split;
#[cfg(test)]
pub(crate) use snapshot::Snapshot;
mod source;
mod state;
mod systems;
mod vk;

pub use action::Action;
pub(crate) use bindings::Source;
pub use presentation::{InputDevice, InputPresentation};
pub use slots::Control;
pub use split::{CoopDevice, PlayerDevice, SplitInput, coop_seats, start_press};
pub use state::{ActionState, Bindings};
pub(crate) use systems::update_actions;
pub use systems::{InputLayerPlugin, MouseCapture, UiFocus};
