//! The input layer: devices -> actions.
//!
//! Game code reads [`ActionState`] and never a key code, mouse event or window event. The mapping from
//! devices (keyboard, mouse, gamepad) to [`Action`]s is the [`Bindings`] resource, so controllers and
//! rebinding need no change in the game code. The layer sits on `bevy_input`, which Bevy's own renderer
//! would also use, so it carries over to a full Bevy move.

mod action;
mod bindings;
mod describe;
mod snapshot;
mod state;
mod systems;

pub use action::Action;
pub use state::{ActionState, Bindings};
pub use systems::{InputLayerPlugin, MouseCapture, UiFocus};
