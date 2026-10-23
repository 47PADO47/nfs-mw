//! Session controls and honest readbacks for matching a reference frame.
use super::*;
use crate::viewer::SunMode;

pub(super) fn show(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &mut Settings) {
    ui.separator();
    ui.heading("Time & weather");
    ui.checkbox(&mut lab.options.enhanced_sky, "Enhanced sky")
        .on_hover_text("Atmospheric colour, sun disk and wind-driven cloud volume. Shared by the main view and reflections. Disable for the authored sky.");
    ui.small("Cloud coverage and sunlight follow the weather and sun below. Full night lighting is not implemented.");
    sky_tuning(ui, lab, settings);
    let Some(mut value) = lab.environment else {
        ui.weak("Enter the world to change time and weather.");
        return;
    };
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.label("Sun:");
        for (mode, label) in [
            (SunMode::CustomClock, "Custom clock"),
            (SunMode::OriginalCycle, "Original cycle"),
            (SunMode::FixedPhase, "Fixed phase"),
        ] {
            changed |= ui.selectable_value(&mut value.sun_mode, mode, label).changed();
        }
    });
    ui.add_enabled_ui(value.sun_mode == SunMode::CustomClock, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (label, hour) in [
                ("Dawn", 6.0),
                ("Morning", 9.0),
                ("Noon", 12.0),
                ("Dusk", 18.0),
                ("Custom night (dusk lighting)", 22.0),
            ] {
                if ui.button(label).clicked() {
                    value.hour = hour;
                    changed = true;
                }
            }
        });
        changed |= ui
            .add(egui::Slider::new(&mut value.hour, 0.0..=23.99).text("Clock hour").custom_formatter(|hour, _| {
                let minutes = (hour * 60.0).round() as u32 % 1440;
                format!("{:02}:{:02}", minutes / 60, minutes % 60)
            }))
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut value.seconds_per_hour, 10.0..=3600.0)
                    .logarithmic(true)
                    .text("Seconds / game hour"),
            )
            .changed();
    });
    ui.add_enabled_ui(value.sun_mode == SunMode::FixedPhase, |ui| {
        changed |=
            ui.add(egui::Slider::new(&mut value.sun_phase, 0.05..=0.95).fixed_decimals(4).text("Sun phase")).changed();
    });
    ui.checkbox(&mut lab.options.freeze_sun, "Freeze time");
    ui.small("Changing Clock hour also advances Enhanced clouds. Freeze time holds sunlight; clouds continue drifting during gameplay.");
    ui.small(
        "Original cycle: 18 minutes each way; its sun never sets. Custom night is a clock extension using dusk lighting, not an original night scene.",
    );
    ui.separator();
    ui.label("Cloud / lighting blend");
    ui.horizontal_wrapped(|ui| {
        if ui.selectable_label(value.cloud_override.is_none(), "Location").clicked() {
            value.cloud_override = None;
            changed = true;
        }
        for (label, amount) in [("Sunny", 0.0), ("Cloudy", 0.5), ("Overcast", 1.0)] {
            if ui.selectable_label(value.cloud_override == Some(amount), label).clicked() {
                value.cloud_override = Some(amount);
                changed = true;
            }
        }
    });
    if let Some(mut amount) = value.cloud_override
        && ui.add(egui::Slider::new(&mut amount, 0.0..=1.0).text("Cloud amount")).changed()
    {
        value.cloud_override = Some(amount);
        changed = true;
    }
    if ui.button("Settle clouds to target").clicked() {
        value.settle_clouds = true;
        changed = true;
    }
    ui.small("Cloud presets apply immediately for comparison. Location resumes authored easing; clouds and rain are independent overrides.");
    ui.horizontal(|ui| {
        ui.label("Forced rain:");
        ui.selectable_value(&mut settings.always_rain, false, "No rain");
        if ui.selectable_value(&mut settings.always_rain, true, "Rain").clicked() && settings.rain_amount.0 == 0 {
            settings.rain_amount.0 = 100;
        }
    });
    ui.add_enabled_ui(settings.always_rain, |ui| {
        ui.add(egui::Slider::new(&mut settings.rain_amount.0, 0..=100).text("Rain particles %"));
    });
    let [x, y, z] = value.sun_direction;
    ui.separator();
    ui.label("Road wetness (comparison override)");
    ui.horizontal_wrapped(|ui| {
        if ui.selectable_label(value.road_wetness_override.is_none(), "Rain history").clicked() {
            value.road_wetness_override = None;
            changed = true;
        }
        for (label, amount) in [("Dry", 0.0), ("Damp", 0.5), ("Wet", 1.0)] {
            if ui.selectable_label(value.road_wetness_override == Some(amount), label).clicked() {
                value.road_wetness_override = Some(amount);
                changed = true;
            }
        }
    });
    if let Some(mut amount) = value.road_wetness_override
        && ui.add(egui::Slider::new(&mut amount, 0.0..=1.0).text("Wetness")).changed()
    {
        value.road_wetness_override = Some(amount);
        changed = true;
    }
    ui.monospace(format!("Road wetness {:.3}", value.road_wetness));
    ui.monospace(format!(
        "Sun phase {:.4} · direction ({x:.3}, {y:.3}, {z:.3}) · dusk {:.3}",
        value.sun_phase, value.dusk_blend
    ));
    ui.monospace(format!(
        "Cloud actual {:.3} → target {:.3} · rain actual {:.3}",
        value.cloud_intensity, value.cloud_target, value.rain_intensity
    ));
    if !value.authored_clouds {
        ui.colored_label(egui::Color32::YELLOW, "Authored cloud map unavailable: Location uses the clear fallback.");
    }
    ui.small("Original-weather gaps: regional storms and tunnel exposure. No rain does not force sunny clouds; full night is a separate custom feature. Road wetness comparison overrides are session-only.");
    if changed {
        lab.environment = Some(value);
        lab.environment_change = Some(value);
    }
}

fn sky_tuning(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &Settings) {
    let defaults = super::profiles::defaults(settings).appearance.lighting;
    ui.collapsing("Sky tuning", |ui| {
        ui.add_enabled_ui(lab.options.enhanced_sky, |ui| {
            let o = &mut lab.options;
            super::controls::slider(ui, &mut o.sky_wind_speed, 0.0..=12.0, "Cloud speed", defaults.sky_wind_speed)
                .on_hover_text("Drift and detail motion. 1 is the baseline; 2 is the default. Cloudier weather adds more wind. 0 holds cloud position, including when seeking the clock. Changing speed repositions the cloud field.");
            super::controls::slider(ui, &mut o.sky_sun_size, 0.5..=3.0, "Sun size", defaults.sky_sun_size)
                .on_hover_text("Visible disk diameter relative to 0.53 degrees. Does not change shadow softness or add lens glare.");
            super::controls::slider(ui, &mut o.sky_sun_brightness, 0.0..=3.0, "Sun brightness", defaults.sky_sun_brightness)
                .on_hover_text("Visible solar disk only. Clouds and scenery still block it. Does not change sunlight on the world.");
            super::controls::slider(ui, &mut o.sky_brightness, 0.25..=2.0, "Sky brightness", defaults.sky_brightness)
                .on_hover_text("Sky and cloud brightness in the main view and reflections. HDR retains bright radiance; SDR compresses highlights.");
            super::controls::slider(ui, &mut o.sky_sunset_warmth, 0.0..=2.0, "Sunrise / sunset warmth", defaults.sky_sunset_warmth)
                .on_hover_text("Extra orange colour near the low sun and on clouds. 0 removes the added tint. World lighting and fog retain their current settings.");
        });
        ui.small("Changes preview immediately. Use Save for startup or Save as… above to keep them.");
    });
}
