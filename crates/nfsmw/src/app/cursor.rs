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
    if garage.menu {
        return;
    }
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
    mut lab: Option<ResMut<crate::devtools::GraphicsLab>>,
    window: Single<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    fe: Option<Res<crate::frontend::Frontend>>,
    mut was_focused: Local<bool>,
    mut exit: MessageWriter<AppExit>,
) {
    if host.renderer.is_none() {
        return;
    }
    let (window, mut cursor) = window.into_inner();
    let uses_mouse = host.scene.captures_mouse();
    garage.had_input = garage.open;
    if host.scene.take_garage_request() {
        garage.menu_requested = true;
    }
    // Normal sessions belong to the FEng front end. F6 never opens over a transaction.
    if garage.menu || garage.menu_requested {
        garage.start_requested = false;
        actions.consume(Action::Garage);
        capture.0 = false;
        focus.0 = false;
        actions.capture_menu();
    }
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
        if let Some(lab) = lab.as_mut() {
            lab.close(&mut capture, &mut focus);
        }
        if !garage.menu {
            toggle_garage(&mut host, &mut garage, &mut capture, &mut focus);
        }
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
    } else if uses_mouse
        && !garage.menu
        && !console.open
        && !focus.0
        && !garage.had_input
        && mouse.just_pressed(MouseButton::Left)
    {
        capture.0 = true;
    }
    if *was_focused && !window.focused {
        capture.0 = false;
    }
    if !window.focused {
        capture.0 = false;
    }
    *was_focused = window.focused;

    // The map draws its own cross over its picture; the system pointer shows everywhere else, so it can click the buttons.
    let on_map_picture = fe.is_some_and(|fe| crate::frontend::map_pointer_on_picture(&fe, window));
    // A scene that does not take the mouse (the map, the menus, a pause) gets the pointer back.
    if !uses_mouse {
        capture.0 = false;
    }
    let (grab_mode, visible) =
        if capture.0 { (CursorGrabMode::Locked, false) } else { (CursorGrabMode::None, !on_map_picture) };
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
    garage.menu = false;
    garage.set_open(true, capture, focus);
}
