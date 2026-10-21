//! Explicit, session-only pursuit controls for collision and damage testing.

use super::{GraphicsLab, Host};

pub(super) fn show(ui: &mut egui::Ui, request: &mut Option<u8>, in_world: bool) {
    ui.collapsing("Pursuit testing", |ui| {
        ui.small(
            "Start a chase at a selected heat to test its police cars. This does not change saved heat progression.",
        );
        ui.add_enabled_ui(in_world, |ui| {
            ui.horizontal_wrapped(|ui| {
                for heat in 1..=10 {
                    if ui.button(format!("Heat {heat}")).clicked() {
                        *request = Some(heat);
                    }
                }
                if ui.button("End pursuit").clicked() {
                    *request = Some(0);
                }
            });
        });
        ui.small("Start driving first. Console alternative: F12, then pursuit 3 or pursuit off.");
    });
}

pub(super) fn apply(lab: &mut GraphicsLab, host: &mut Host) {
    let Some(heat) = lab.pursuit_request.take() else { return };
    let Some(renderer) = host.renderer.as_mut() else {
        lab.message = Some("Pursuit controls need a running game renderer".into());
        return;
    };
    let argument = match heat {
        0 => "off".to_owned(),
        value => value.to_string(),
    };
    let result = host.scene.command(renderer.as_mut(), "pursuit", &[&argument]);
    lab.message = Some(match result {
        Some(Ok(message) | Err(message)) => message,
        None => "Pursuit controls are available while driving in the world".into(),
    });
}
