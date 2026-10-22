//! The Start press of the split-screen Press Start screens: the input layer names the device whose Start or accept key
//! went down (`input::SplitInput::start_press`), the plugin hands it over each frame, and the top screen hears it.

use super::{Command, Env, Screens};
use crate::input::PlayerDevice;

impl Screens {
    /// Sets the device whose Start went down this frame; the next [`Screens::update`] gives it to the top screen.
    pub fn set_start_press(&mut self, device: Option<PlayerDevice>) {
        self.start_press = device;
    }

    /// Hands the frame's Start press to the top screen, unless the screen is still waiting for the buttons to be let go.
    pub(super) fn deliver_start_press(&mut self, env: &mut Env, out: &mut Vec<Command>) {
        let Some(device) = self.start_press.take() else { return };
        if self.await_release {
            return;
        }
        let Some(top) = self.stack.last().map(|s| s.id) else { return };
        self.with_logic(top, env, out, |logic, cx| logic.start_pressed(cx, device));
    }
}
