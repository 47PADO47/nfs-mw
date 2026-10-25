//! F5 input ownership and screenshot shortcuts.
use crate::extra_options::Action;
use crate::input::{MouseCapture, UiFocus};
use bevy_app::{App, PreUpdate};
use bevy_ecs::prelude::*;
use bevy_input::{ButtonInput, InputSystems, keyboard::KeyCode};
#[derive(Resource, Default)]
pub(super) struct Overlay {
    pub open: bool,
    restore_capture: bool,
    active: bool,
}
pub(super) fn build(app: &mut App) {
    app.insert_resource(Overlay {
        open: std::env::var("NFSMW_EXTRA_OVERLAY").as_deref() == Ok("1"),
        ..Default::default()
    })
    .add_systems(
        PreUpdate,
        toggle.after(InputSystems).after(super::graphics_lab::toggle).before(crate::input::update_actions),
    );
}
fn key(name: &str) -> KeyCode {
    match name {
        "Insert" => KeyCode::Insert,
        "End" => KeyCode::End,
        "F3" => KeyCode::F3,
        "F2" => KeyCode::F2,
        "F4" => KeyCode::F4,
        _ => KeyCode::Home,
    }
}
#[allow(clippy::too_many_arguments)]
fn toggle(
    mut overlay: ResMut<Overlay>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut host: NonSendMut<crate::app::Host>,
    mut capture: ResMut<MouseCapture>,
    mut focus: ResMut<UiFocus>,
    mut garage: ResMut<crate::garage::Overlay>,
    mut lab: ResMut<super::GraphicsLab>,
    mut console: ResMut<super::Console>,
) {
    if keys.just_pressed(KeyCode::F5) && !garage.menu {
        overlay.open = !overlay.open;
        if let Some(state) = host.scene.extra_options() {
            state.hide_ui = false;
        }
        if overlay.open {
            garage.set_open(false, &mut capture, &mut focus);
            lab.close(&mut capture, &mut focus);
            console.set_open(false, &mut capture, &mut focus);
        }
        keys.reset(KeyCode::F5);
    }
    if keys.just_pressed(KeyCode::F6)
        || keys.just_pressed(KeyCode::F8)
        || keys.just_pressed(KeyCode::F12)
        || garage.menu
    {
        overlay.open = false;
        if let Some(state) = host.scene.extra_options() {
            state.hide_ui = false;
        }
    }
    if overlay.open && keys.just_pressed(KeyCode::Escape) {
        overlay.open = false;
        keys.reset(KeyCode::Escape);
    }
    if !console.open
        && !garage.open
        && !garage.menu
        && !lab.open
        && let Some(state) = host.scene.extra_options()
    {
        let hide = key(&state.camera.hide_ui_key);
        if keys.just_pressed(hide) {
            state.hide_ui = !state.hide_ui;
            if state.hide_ui {
                overlay.open = false;
            }
            keys.reset(hide);
        }
        let center = key(&state.camera.center_key);
        if !overlay.open && keys.just_pressed(center) {
            keys.reset(center);
            let host = &mut *host;
            if let Some(renderer) = host.renderer.as_mut() {
                let _ = host.scene.apply_extra_option(renderer.as_mut(), Action::Center);
            }
        }
    }
    overlay.sync(&mut capture, &mut focus);
    if lab.open || console.open {
        capture.0 = false;
        focus.0 = true;
    }
}

impl Overlay {
    pub(super) fn sync(&mut self, capture: &mut MouseCapture, focus: &mut UiFocus) {
        if self.open {
            if !self.active {
                self.restore_capture = capture.0;
            }
            self.active = true;
            capture.0 = false;
            focus.0 = true;
            return;
        }
        if self.active {
            capture.0 = self.restore_capture;
            focus.0 = false;
            self.active = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn panel_close_restores_capture_and_drops_ui_focus() {
        let mut overlay = Overlay { open: true, ..Default::default() };
        let mut capture = MouseCapture(true);
        let mut focus = UiFocus(false);
        overlay.sync(&mut capture, &mut focus);
        assert!(!capture.0 && focus.0);
        overlay.open = false;
        overlay.sync(&mut capture, &mut focus);
        assert!(capture.0 && !focus.0);
        overlay.sync(&mut capture, &mut focus);
        assert!(capture.0 && !focus.0);
    }
}
