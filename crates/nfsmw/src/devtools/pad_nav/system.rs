//! The Bevy side of the controller's navigation.

use bevy_ecs::prelude::*;
use bevy_input::gamepad::{Gamepad, GamepadAxis, GamepadButton};
use bevy_time::{Real, Time};

use super::state::{Button, Direction, Nav, PadInput, PadNav};
use crate::devtools::panels::{Panels, Request};
use crate::gui::Gui;

/// The controller buttons the panels read, as `Gamepad` reports them.
const BUTTONS: [(GamepadButton, Button); 10] = [
    (GamepadButton::DPadUp, Button::Up),
    (GamepadButton::DPadDown, Button::Down),
    (GamepadButton::DPadLeft, Button::Left),
    (GamepadButton::DPadRight, Button::Right),
    (GamepadButton::South, Button::A),
    (GamepadButton::East, Button::B),
    (GamepadButton::LeftTrigger, Button::LeftBumper),
    (GamepadButton::RightTrigger, Button::RightBumper),
    (GamepadButton::LeftThumb, Button::LeftStick),
    (GamepadButton::RightThumb, Button::RightStick),
];

/// What every connected controller is doing: a button is held when any of them holds it, the stick is the largest push.
fn read(pads: &Query<&Gamepad>) -> PadInput {
    let mut input = PadInput::default();
    for pad in pads {
        for (code, button) in BUTTONS {
            if pad.pressed(code) {
                input.held.insert(button);
            }
        }
        let axis = |axis: GamepadAxis| pad.get(axis).unwrap_or(0.0);
        let (x, y) = (axis(GamepadAxis::LeftStickX), axis(GamepadAxis::LeftStickY));
        if x.abs() > input.stick.0.abs() {
            input.stick.0 = x;
        }
        if y.abs() > input.stick.1.abs() {
            input.stick.1 = y;
        }
    }
    input
}

fn key_events(key: egui::Key, shift: bool) -> [egui::Event; 2] {
    let modifiers = egui::Modifiers { shift, ..egui::Modifiers::NONE };
    let event = |pressed| egui::Event::Key { key, physical_key: Some(key), pressed, repeat: false, modifiers };
    [event(true), event(false)]
}

fn arrow(direction: Direction) -> egui::Key {
    match direction {
        Direction::Up => egui::Key::ArrowUp,
        Direction::Down => egui::Key::ArrowDown,
        Direction::Left => egui::Key::ArrowLeft,
        Direction::Right => egui::Key::ArrowRight,
    }
}

/// The egui events that make `nav` and what it asks of the panels. `focused`: some widget has the keyboard focus; without one
/// the arrows cannot start a move (egui moves a focus, it does not give one), so they hand the focus to the first widget (down,
/// right) or the last (up, left). `escape`: a popup is open or a text field has the keyboard, which B leaves before it closes a panel.
fn translate(nav: Nav, focused: bool, escape: bool, events: &mut Vec<egui::Event>) -> Option<Request> {
    match nav {
        Nav::Chord => return Some(Request::Toggle),
        Nav::Back if escape => events.extend(key_events(egui::Key::Escape, false)),
        Nav::Back => return Some(Request::Back),
        Nav::Activate => events.extend(key_events(egui::Key::Enter, false)),
        Nav::Tab(forward) => events.extend(key_events(egui::Key::Tab, !forward)),
        Nav::Arrow(direction) if focused => events.extend(key_events(arrow(direction), false)),
        Nav::Arrow(direction) => {
            let forward = matches!(direction, Direction::Down | Direction::Right);
            events.extend(key_events(egui::Key::Tab, !forward));
        }
    }
    None
}

/// Reads the controllers, once a frame, before the panels are served: the chord and the back button become requests, the
/// rest egui events for the next UI pass. The `devtools press` presses of the console come through the same way.
pub(in crate::devtools) fn collect(
    pads: Query<&Gamepad>,
    time: Res<Time<Real>>,
    gui: Res<Gui>,
    mut nav: Local<PadNav>,
    mut panels: Panels,
) {
    let _trace = blackbox_trace::scope("Devtools: controller");
    let input = read(&pads);
    panels.state.pad = pads.iter().next().is_some();
    let up = panels.which().is_some() || panels.state.any();
    let mut navs = nav.step(&input, time.delta_secs(), up);
    navs.extend(std::mem::take(&mut panels.state.pressed).into_iter().filter_map(super::state::press));
    if navs.is_empty() {
        return;
    }
    let focused = gui.ctx.memory(|memory| memory.focused().is_some());
    let escape = egui::Popup::is_any_open(&gui.ctx) || gui.ctx.egui_wants_keyboard_input();
    for nav in navs {
        let mut events = Vec::new();
        let request = translate(nav, focused, escape, &mut events);
        panels.state.events.extend(events);
        if let Some(request) = request {
            panels.state.request(request);
        }
    }
}

/// Keeps the widget that has the keyboard focus in view: at the end of the content of a scroll area, scrolls to the focused widget
/// when it lies in that content, is not in view and the focus has just moved there. Egui moves the focus but does not scroll to
/// it, and a focus that stays where it is must not pull the view back from a scroll of the mouse.
pub(in crate::devtools) fn follow_focus(ui: &egui::Ui) {
    let ctx = ui.ctx();
    let Some(focused) = ctx.memory(|memory| memory.focused()) else { return };
    let seen = egui::Id::new("devtools_pad_focus_seen");
    if ctx.data(|data| data.get_temp::<egui::Id>(seen)) == Some(focused) {
        return;
    }
    let Some(response) = ctx.read_response(focused) else { return };
    if !ui.min_rect().contains_rect(response.rect) {
        return;
    }
    ctx.data_mut(|data| data.insert_temp(seen, focused));
    if ui.clip_rect().contains_rect(response.rect) {
        return;
    }
    ui.scroll_to_rect(response.rect, None);
}
