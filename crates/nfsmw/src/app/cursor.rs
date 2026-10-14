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
    garage: &mut crate::garage::Overlay,
    capture: &mut MouseCapture,
    focus: &mut UiFocus,
    exit: &mut MessageWriter<AppExit>,
) {
    if garage.open {
        garage.set_open(false, capture, focus);
        return;
    }
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
    mut host: NonSendMut<Host>,
    mut actions: ResMut<ActionState>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut capture: ResMut<MouseCapture>,
    mut focus: ResMut<UiFocus>,
    mut console: ResMut<Console>,
    mut garage: ResMut<crate::garage::Overlay>,
    window: Single<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut was_focused: Local<bool>,
    mut exit: MessageWriter<AppExit>,
) {
    if host.renderer.is_none() {
        return;
    }
    let (window, mut cursor) = window.into_inner();
    let uses_mouse = host.scene.captures_mouse();
    garage.had_input = garage.open;
    if garage.start_requested && !console.open && !host.scene.paused() && host.scene.garage().is_some() {
        if let Some(model) = host.scene.garage() {
            model.options_tab = garage.start_tab;
            model.preview = garage.start_tab == crate::garage::OptionsTab::Parts;
        }
        toggle_garage(&mut host, &mut garage, &mut capture, &mut focus);
        garage.start_requested = false;
        actions.consume(Action::Garage);
    }
    if actions.just_pressed(Action::Garage) && !console.open {
        toggle_garage(&mut host, &mut garage, &mut capture, &mut focus);
    }
    if garage.had_input {
        actions.capture_ui();
        actions.consume(Action::Console);
    }
    if actions.just_pressed(Action::Cancel) {
        let garage_had_it = garage.open || garage.had_input;
        on_cancel(&host, &mut console, &mut garage, &mut capture, &mut focus, &mut exit);
        if garage_had_it {
            actions.consume(Action::Cancel);
        }
    } else if uses_mouse && !console.open && !garage.had_input && mouse.just_pressed(MouseButton::Left) {
        capture.0 = true;
    }
    if *was_focused && !window.focused {
        capture.0 = false;
    }
    if !window.focused {
        capture.0 = false;
    }
    *was_focused = window.focused;

    let (grab_mode, visible) = if capture.0 { (CursorGrabMode::Locked, false) } else { (CursorGrabMode::None, true) };
    if cursor.grab_mode != grab_mode || cursor.visible != visible {
        cursor.grab_mode = grab_mode;
        cursor.visible = visible;
    }
}

fn toggle_garage(
    host: &mut Host,
    garage: &mut crate::garage::Overlay,
    capture: &mut MouseCapture,
    focus: &mut UiFocus,
) {
    if garage.open {
        garage.set_open(false, capture, focus);
        return;
    }
    if host.scene.paused() {
        return;
    }
    let Some(model) = host.scene.garage() else { return };
    model.reset_draft();
    garage.set_open(true, capture, focus);
}
