//! Mouse capture for mouse-look scenes: the cursor is hidden and held in the window, so mouse motion
//! turns the camera without holding a button. The first Esc releases it, the next one quits, a click
//! captures it again, and losing focus releases it.

use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use bevy_input::ButtonInput;
use bevy_input::mouse::MouseButton;
use bevy_window::{CursorGrabMode, CursorOptions, PrimaryWindow, Window};

use super::host::Host;
use crate::devtools::Console;
use crate::input::{Action, ActionState, MouseCapture, UiFocus};

/// Escape: close the console; else leave it to the front end if that uses Escape (pause, back); else release the
/// mouse; else quit.
fn on_cancel(
    host: &Host,
    console: &mut Console,
    capture: &mut MouseCapture,
    focus: &mut UiFocus,
    exit: &mut MessageWriter<AppExit>,
) {
    if console.open {
        console.set_open(false, capture, focus);
        return;
    }
    if host.cancel_handled {
        return;
    }
    if capture.0 {
        capture.0 = false;
        return;
    }
    exit.write(AppExit::Success);
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    host: NonSend<Host>,
    actions: Res<ActionState>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut capture: ResMut<MouseCapture>,
    mut focus: ResMut<UiFocus>,
    mut console: ResMut<Console>,
    window: Single<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut was_focused: Local<bool>,
    mut exit: MessageWriter<AppExit>,
) {
    if host.renderer.is_none() {
        return;
    }
    let (window, mut cursor) = window.into_inner();
    let uses_mouse = host.scene.captures_mouse();
    if actions.just_pressed(Action::Cancel) {
        on_cancel(&host, &mut console, &mut capture, &mut focus, &mut exit);
    } else if uses_mouse && !console.open && mouse.just_pressed(MouseButton::Left) {
        capture.0 = true;
    }
    if *was_focused && !window.focused {
        capture.0 = false;
    }
    *was_focused = window.focused;

    let (grab_mode, visible) = if capture.0 { (CursorGrabMode::Locked, false) } else { (CursorGrabMode::None, true) };
    if cursor.grab_mode != grab_mode || cursor.visible != visible {
        cursor.grab_mode = grab_mode;
        cursor.visible = visible;
    }
}
