use super::*;
pub(super) fn show(ui: &mut egui::Ui, state: &mut State, settings: &mut Settings, actions: &mut Vec<Action>) {
    let before = state.camera.clone();
    let mode = settings.camera_mode;
    egui::ComboBox::from_id_salt("extra_camera_mode").selected_text(camera_name(mode)).show_ui(ui, |ui| {
        for mode in CameraMode::ALL {
            ui.selectable_value(&mut settings.camera_mode, mode, camera_name(mode));
        }
    });
    ui.small("Original extra views are directly selectable, even when normal camera cycling skips them.");
    ui.separator();
    ui.checkbox(&mut state.camera.auto_center, "Automatically center after looking around");
    ui.add(egui::Slider::new(&mut state.camera.center_delay, 0.0..=30.0).text("Center delay (s)"));
    ui.add(egui::Slider::new(&mut state.camera.center_speed, 0.1..=20.0).text("Center speed"));
    ui.small("Centering controls apply to Remake chase cameras. Use freecam for unrestricted original-camera shots.");
    if ui.button("Center camera now").clicked() {
        actions.push(Action::Center);
    }
    egui::ComboBox::from_id_salt("center_shortcut")
        .selected_text(format!("Center shortcut: {}", state.camera.center_key))
        .show_ui(ui, |ui| {
            for key in ["Home", "Insert", "End"] {
                ui.selectable_value(&mut state.camera.center_key, key.into(), key);
            }
        });
    ui.separator();
    ui.label("Remake chase adjustments · live");
    ui.add(egui::Slider::new(&mut state.camera.distance, 1.0..=30.0).text("Distance (m)"));
    ui.add(egui::Slider::new(&mut state.camera.height, 0.0..=10.0).text("Height (m)"));
    ui.add(egui::Slider::new(&mut state.camera.fov, 20.0..=120.0).text("Vertical field of view"));
    ui.checkbox(&mut state.camera.speed_zoom, "Change distance and FOV with speed");
    ui.checkbox(&mut state.camera.preserve_framing, "Keep car position when swapping with a bookmarked/free camera");
    if ui.button("Reset camera preferences").clicked() {
        state.camera = Default::default();
    }
    ui.separator();
    let free_label = match state.freecam {
        true => "Return to driving camera",
        false => "Detach free camera from current view",
    };
    if ui.button(free_label).clicked() {
        actions.push(Action::Freecam);
    }
    if state.freecam {
        let fov = state.free_fov.get_or_insert(70.0);
        ui.add(egui::Slider::new(fov, 20.0..=120.0).text("Free camera FOV"));
    }
    ui.small("Freecam: WASD, Space/C up/down, Shift faster, mouse/right stick to look, wheel for speed.");
    ui.checkbox(&mut state.testing.paused, "Pause simulation · camera remains usable");
    ui.checkbox(&mut state.hide_ui, "Hide all UI for screenshots");
    egui::ComboBox::from_id_salt("hide_shortcut")
        .selected_text(format!("Show/hide UI shortcut: {}", state.camera.hide_ui_key))
        .show_ui(ui, |ui| {
            for key in ["F4", "F3", "F2"] {
                ui.selectable_value(&mut state.camera.hide_ui_key, key.into(), key);
            }
        });
    ui.small("F5 always restores the UI. Preferences save automatically; freecam and pause are session-only.");
    if before != state.camera {
        state.message =
            crate::extra_options::storage::save(state).err().map(|e| format!("Preferences not saved: {e:#}"));
    }
    if mode != settings.camera_mode {
        state.message =
            Settings::save(&crate::settings::Partial { camera_mode: Some(settings.camera_mode), ..Default::default() })
                .err()
                .map(|e| e.to_string());
    }
}
