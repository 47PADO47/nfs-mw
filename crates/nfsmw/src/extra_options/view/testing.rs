use super::*;
pub(super) fn show(ui: &mut egui::Ui, state: &mut State, settings: &mut Settings, actions: &mut Vec<Action>) {
    ui.label("Physics update rate");
    let before = settings.physics_rate_hz;
    egui::ComboBox::from_id_salt("physics_update_rate")
        .selected_text(crate::settings::physics_rate::label(settings.physics_rate_hz))
        .show_ui(ui, |ui| {
            for rate in [60, 144] {
                ui.selectable_value(&mut settings.physics_rate_hz, rate, crate::settings::physics_rate::label(rate));
            }
        });
    ui.small("Rendering FPS is independent. The 144 Hz comparison mode changes numerical handling.");
    ui.small("This preference is saved. Multiplayer always uses 60 Hz.");
    if before != settings.physics_rate_hz {
        state.message = Settings::save(&crate::settings::Partial {
            physics_rate_hz: Some(settings.physics_rate_hz),
            ..Default::default()
        })
        .err()
        .map(|e| e.to_string());
    }
    ui.separator();
    if let Some(mut env) = state.environment {
        ui.collapsing("Weather and time · same controls as F8", |ui| {
            let before = env;
            ui.horizontal_wrapped(|ui| {
                for (mode, name) in [
                    (crate::viewer::SunMode::CustomClock, "Custom clock"),
                    (crate::viewer::SunMode::OriginalCycle, "Original cycle"),
                    (crate::viewer::SunMode::FixedPhase, "Fixed phase"),
                ] {
                    ui.selectable_value(&mut env.sun_mode, mode, name);
                }
            });
            ui.add_enabled_ui(env.sun_mode == crate::viewer::SunMode::CustomClock, |ui| {
                ui.add(egui::Slider::new(&mut env.hour, 0.0..=23.99).text("Custom clock hour"));
            });
            ui.add_enabled_ui(env.sun_mode == crate::viewer::SunMode::FixedPhase, |ui| {
                ui.add(egui::Slider::new(&mut env.sun_phase, 0.05..=0.95).text("Original sun phase"));
            });
            ui.small("The original sun cycle never sets. Custom night currently uses dusk lighting.");
            let mut clouds = env.cloud_override.is_some();
            if ui.checkbox(&mut clouds, "Override cloud cover").changed() {
                env.cloud_override = clouds.then_some(env.cloud_intensity);
            }
            if let Some(value) = env.cloud_override.as_mut() {
                ui.add(egui::Slider::new(value, 0.0..=1.0).text("Cloud cover"));
            }
            if env != before {
                actions.push(Action::Environment(env));
            }
        });
    }
    ui.label("Session-only overrides · apply live");
    ui.checkbox(&mut state.testing.traffic_off, "Traffic off · remove ambient cars immediately");
    ui.small("Traffic density remains in F8. Turning this off restores that density. Police and race opponents are separate.");
    ui.checkbox(&mut state.testing.police_off, "Disable police · remove police and prevent new pursuits");
    ui.horizontal(|ui| {
        ui.add(egui::Slider::new(&mut state.testing.heat, 1..=10).text("Heat"));
        ui.checkbox(&mut state.testing.lock_heat, "Lock heat");
    });
    ui.horizontal(|ui| {
        if ui.add_enabled(!state.testing.police_off, egui::Button::new("Start pursuit")).clicked() {
            actions.push(Action::StartPursuit);
        }
        if ui.add_enabled(!state.testing.police_off, egui::Button::new("Set current heat")).clicked() {
            actions.push(Action::SetHeat);
        }
        if ui.button("End current pursuit").clicked() {
            actions.push(Action::EndPursuit);
        }
    });
    ui.checkbox(&mut state.testing.unlimited_nos, "Unlimited NOS · requires an installed NOS system");
    ui.checkbox(&mut state.testing.unlimited_breaker, "Unlimited Speedbreaker · normal activation rules apply");
    ui.horizontal(|ui| {
        if ui.button("Refill NOS + Speedbreaker").clicked() {
            actions.push(Action::Refill);
        }
        if ui.button("Recover car to road").clicked() {
            actions.push(Action::Recover);
        }
        if ui.button("Repair car").clicked() {
            actions.push(Action::Repair);
        }
    });
    ui.checkbox(&mut state.testing.paused, "Pause simulation · camera remains usable");
    if ui.button("Clear all testing overrides").clicked() {
        actions.push(Action::Clear);
    }
    ui.small("Available in local career and free roam. Overrides last for this session; career progress still saves normally. Multiplayer and co-op are unavailable.");
}
