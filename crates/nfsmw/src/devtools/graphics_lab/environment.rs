//! Session controls and honest readbacks for matching a reference frame.
use super::*;
use crate::viewer::SunMode;

pub(super) fn show(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &mut Settings) {
    ui.separator();
    ui.heading("Time & weather");
    ui.checkbox(&mut lab.options.enhanced_sky, "Enhanced sky")
        .on_hover_text("Atmospheric colour, sun disk and wind-driven cloud volume. Shared by the main view and reflections. Disable for the authored sky.");
    ui.add_enabled_ui(lab.options.enhanced_sky, |ui| {
        ui.checkbox(&mut lab.options.cloud_shadows, "Cloud shadows")
            .on_hover_text("Moving cloud shade on sunlight. Independent of geometry shadows; requires Enhanced sky.");
    });
    let defaults = super::profiles::defaults(settings).appearance.lighting;
    let mut cycle = lab.options.full_day_night;
    if super::controls::checkbox(ui, &mut cycle, "Full day/night cycle", defaults.full_day_night)
        .on_hover_text("Runs on the custom clock, overriding the other sun modes, with an artistic sun and moon. Switches local lamps on; turning the cycle off restores them.")
        .changed()
    {
        set_full_cycle(lab, cycle);
    }
    ui.add_enabled_ui(lab.options.full_day_night, |ui| {
        super::controls::checkbox(ui, &mut lab.options.auto_player_lights, "Automatic player lights", defaults.auto_player_lights)
            .on_hover_text("Switch headlights and running lamps at dusk and dawn. You can still toggle them manually between transitions.");
    });
    let night_changed = super::controls::checkbox(
        ui,
        &mut lab.options.experimental_night,
        "Experimental moonlit night",
        defaults.experimental_night,
    )
    .on_hover_text("Dims and cools world lighting and enables headlights. Authored light remains in the textures.")
    .changed();
    if night_changed && lab.options.experimental_night {
        set_full_cycle(lab, false);
        lab.options.local_lamps = true;
        lab.options.local_street_lights = true;
        lab.options.local_headlights = true;
        lab.options.local_traffic_headlights = true;
    }
    ui.add_enabled_ui(lab.options.experimental_night || lab.options.full_day_night, |ui| {
        super::controls::slider(
            ui,
            &mut lab.options.night_sky_brightness,
            0.05..=0.4,
            "Night sky brightness",
            defaults.night_sky_brightness,
        )
        .on_hover_text("Sky and cloud brightness at night. Used by the full cycle and the manual night preview.");
    });
    ui.small("The manual night switch is a fixed preview; the full cycle replaces it.");
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
            if ui.selectable_value(&mut value.sun_mode, mode, label).changed() {
                changed = true;
                if mode != SunMode::CustomClock {
                    set_full_cycle(lab, false);
                }
            }
        }
    });
    let night_label = match lab.options.full_day_night {
        true => "Night",
        false => "Night (dusk lighting)",
    };
    ui.add_enabled_ui(value.sun_mode == SunMode::CustomClock, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (label, hour) in [("Dawn", 6.0), ("Morning", 9.0), ("Noon", 12.0), ("Dusk", 18.0), (night_label, 22.0)]
            {
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
        "Original cycle: 18 minutes each way; its sun never sets. Without the full cycle, custom-clock hours after dusk keep dusk lighting; the original game has no night scene.",
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
        ui.label("Rain:");
        ui.selectable_value(&mut settings.always_rain, false, "Location rain");
        if ui.selectable_value(&mut settings.always_rain, true, "Rain").clicked() && settings.rain_amount.0 == 0 {
            settings.rain_amount.0 = 100;
        }
    });
    ui.add_enabled_ui(settings.always_rain, |ui| {
        ui.add(egui::Slider::new(&mut settings.rain_amount.0, 0..=100).text("Rain particles %"));
    });
    ui.horizontal(|ui| {
        ui.add(egui::Slider::new(&mut settings.rain_droplet_scale, 0.0..=2.0).text("Lens droplet size").suffix("×"))
            .on_hover_text("0 hides camera droplets; 0.5 is the smaller default; 1 matches the reconstructed PC size. Falling rain and tyre spray are unchanged.");
        super::controls::reset(ui, &mut settings.rain_droplet_scale, 0.5, "Lens droplet size");
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
    ui.small("Location rain follows the original regional weather map. Rain forces precipitation; 0% disables it. Cloud and road-wetness overrides are separate, session-only comparisons. Full night is a custom feature.");
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

/// The local lamp options the full day/night cycle switches on, as they were before.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct LampOptions([bool; 5]);

fn lamps(o: &mut RenderExperiments) -> [&mut bool; 5] {
    [
        &mut o.local_lamps,
        &mut o.local_street_lights,
        &mut o.local_headlights,
        &mut o.local_traffic_headlights,
        &mut o.local_tail_lights,
    ]
}

/// Turn the full day/night cycle on or off from the lab. On: the manual night ends, the lamps light (their previous
/// values kept) and the sun moves to the custom clock, sent to the world at once. Off: each lamp the cycle switched on
/// returns to its previous value; one switched off meanwhile stays off.
pub(super) fn set_full_cycle(lab: &mut GraphicsLab, on: bool) {
    lab.options.full_day_night = on;
    if !on {
        let Some(previous) = lab.cycle_lamp_restore.take() else { return };
        for (option, was) in lamps(&mut lab.options).into_iter().zip(previous.0) {
            *option &= was;
        }
        return;
    }
    lab.options.experimental_night = false;
    lab.cycle_lamp_restore = Some(LampOptions(lamps(&mut lab.options).map(|option| *option)));
    for option in lamps(&mut lab.options) {
        *option = true;
    }
    let Some(value) = &mut lab.environment else { return };
    value.sun_mode = SunMode::CustomClock;
    lab.environment_change = Some(*value);
}

#[cfg(test)]
mod cycle_tests {
    use super::*;

    #[test]
    fn the_cycle_takes_the_custom_clock_and_restores_the_lamps_it_lit() {
        let mut lab = GraphicsLab {
            environment: Some(crate::viewer::Environment { sun_mode: SunMode::FixedPhase, ..Default::default() }),
            ..Default::default()
        };
        lab.options.local_lamps = true;
        lab.options.local_street_lights = false;
        lab.options.local_tail_lights = false;
        lab.options.experimental_night = true;
        set_full_cycle(&mut lab, true);
        assert!(lab.options.full_day_night && !lab.options.experimental_night);
        assert!(lamps(&mut lab.options).iter().all(|option| **option));
        assert_eq!(lab.environment_change.map(|value| value.sun_mode), Some(SunMode::CustomClock));
        lab.options.local_lamps = false;
        set_full_cycle(&mut lab, false);
        assert!(!lab.options.full_day_night);
        assert!(!lab.options.local_lamps, "a lamp switched off during the cycle stays off");
        assert!(!lab.options.local_street_lights && !lab.options.local_tail_lights, "lamps the cycle lit go back off");
        assert!(lab.cycle_lamp_restore.is_none());
        let before = lab.options;
        set_full_cycle(&mut lab, false);
        assert_eq!(lab.options, before, "turning an inactive cycle off changes nothing");
    }
}
