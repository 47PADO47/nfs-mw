//! The developer console (F12): a log view and a command line.
//!
//! The console's state is the [`Console`] resource and the log buffer, both plain data. Typed lines are
//! parsed into [`parse::Command`]s and run by [`exec`]; [`view`] is the thin egui panel over all of it.

mod availability;
mod exec;
#[cfg(test)]
mod exhaust_flames_tests;
mod gfx_cmd;
#[cfg(test)]
mod gfx_tests;
#[cfg(test)]
mod hud_layout_tests;
mod parse;
mod settings_cmd;
#[cfg(test)]
mod smoke_quality_sync_tests;
#[cfg(test)]
mod tire_sync_tests;
#[cfg(test)]
mod vehicle_effects_tests;
mod view;

use bevy_app::{App, Update};
use bevy_ecs::prelude::*;

use crate::app::FrameSet;
use crate::input::{Action, ActionState, MouseCapture, UiFocus};

pub use view::show;

#[derive(Resource, Default)]
pub struct Console {
    pub open: bool,
    /// The line being typed.
    input: String,
    history: Vec<String>,
    /// Which history entry Up/Down is showing.
    history_at: Option<usize>,
    /// Lines to run next frame.
    pending: Vec<String>,
    /// The scene's own commands, for `help` and Tab.
    scene_commands: &'static [(&'static str, &'static str)],
    /// The mouse was captured when the console opened; give it back on close.
    restore_capture: bool,
}

impl Console {
    /// Open or close the console. It takes the keyboard and frees the mouse while open.
    pub fn set_open(&mut self, open: bool, capture: &mut MouseCapture, focus: &mut UiFocus) {
        if open == self.open {
            return;
        }
        self.open = open;
        focus.0 = open;
        if open {
            self.restore_capture = capture.0;
            capture.0 = false;
        } else {
            capture.0 = self.restore_capture;
        }
    }
}

/// Queue `exec` for the first frames and optionally open the console (`--exec`, `--open-console`).
pub fn start(app: &mut App, exec: Vec<String>, open: bool) {
    let world = app.world_mut();
    let mut console = world.resource_mut::<Console>();
    console.pending.extend(exec);
    if open {
        console.open = true;
        console.restore_capture = false;
        world.resource_mut::<UiFocus>().0 = true;
    }
}

pub fn build(app: &mut App) {
    app.init_resource::<Console>().add_systems(
        Update,
        (
            toggle.in_set(FrameSet::Prepare).before(crate::app::cursor_update),
            (exec::execute, exec::sync_settings).chain().in_set(FrameSet::Commands),
        ),
    );
}

/// F12 opens and closes the console.
fn toggle(
    mut console: ResMut<Console>,
    actions: Res<ActionState>,
    mut capture: ResMut<MouseCapture>,
    mut focus: ResMut<UiFocus>,
) {
    if actions.just_pressed(Action::Console) {
        let open = !console.open;
        console.set_open(open, &mut capture, &mut focus);
    }
}
