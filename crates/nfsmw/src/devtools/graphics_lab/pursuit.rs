//! Explicit, session-only pursuit controls for collision and damage testing.

use super::{GraphicsLab, Host};

pub(super) fn options(ui: &mut egui::Ui, options: &mut crate::settings::PursuitOptions) {
    ui.collapsing("Police limits (enhancement)", |ui| {
        ui.checkbox(&mut options.enhanced, "Enhanced police limits");
        ui.add_enabled_ui(options.enhanced, |ui| {
            ui.add(egui::Slider::new(&mut options.wave_percent, 100..=400).suffix("%").text("Cops per wave"));
            ui.add(egui::Slider::new(&mut options.max_active_cops, 8..=64).text("Active police car limit"));
            ui.add(egui::Slider::new(&mut options.heat_cap, 0..=10).text("Maximum heat (0: event rules)"));
        });
        ui.small(
            "Off keeps reference limits. Cops per wave scales the heat table's counts (the wave, the cops to evade \
             before backup, the cool-down search) from the next chase. The active limit only caps them: reference \
             waves are 4 to 10 cops, so at 100% no limit above about 12 changes a chase.",
        );
        ui.checkbox(&mut options.bounty_fixes, "Bounty fixes")
            .on_hover_text("Heat 10 takedowns score, a downed helicopter pays 100,000 with its own line, and 2-minute combo windows last 10 s. Off restores the original's bugs.");
        ui.checkbox(&mut options.simple_police_tires, "Original police tires (experimental)").on_hover_text(
            "Police cars drive on the original's simple police tire (steady grip in a slide, very strong brakes) instead of the racer tire. Applies live.",
        );
        ui.small("Heat 6–10 includes special-event rules. The final pursuit stays fixed at heat 6. Changes apply live; Save for startup keeps them.");
    });
}

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
