//! Traffic population, independent of mesh detail and graphics quality presets.
use crate::settings::{PopulationPreset, Settings, TrafficPreset};

pub(super) fn show(ui: &mut egui::Ui, settings: &mut Settings, status: Option<&str>) {
    if let Some(status) = status {
        ui.small(status);
        ui.separator();
    }
    ui.horizontal_wrapped(|ui| {
        ui.label("Traffic range preset:");
        for preset in TrafficPreset::ALL {
            if ui.selectable_label(settings.traffic_preset() == preset, preset.label()).clicked() {
                settings.select_traffic_preset(preset);
            }
        }
    });
    ui.small("Reference follows the playable PC reference. Extended increases range and retention; density and budget stay independent.");
    ui.small("Changes apply live; existing cars recycle naturally. Save profile keeps these settings. Named F8 presets and A/B include traffic.");
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        ui.label("Population preset:");
        for preset in PopulationPreset::ALL {
            if ui.selectable_label(settings.population_preset() == preset, preset.label()).clicked() {
                settings.select_population_preset(preset);
            }
        }
    });
    ui.small("Normal uses original model shares. High / Very High fill common-car slots and replenish them faster; rare vehicles keep their native limits.");
    let before = settings.traffic_policy.range_only();
    ui.add(egui::Slider::new(&mut settings.traffic, 0..=64).text("Active vehicle budget (0: off)"));
    let policy = &mut settings.traffic_policy;
    ui.checkbox(&mut policy.common_fill, "Fill unused slots with common cars (custom)");
    ui.add(egui::Slider::new(&mut policy.common_rate_percent, 100..=300).text("Common replenishment % (custom)"));
    ui.horizontal_wrapped(|ui| {
        ui.label("Density test:");
        for density in [0, 10, 30, 50, 100] {
            if ui.button(format!("{density}%")).clicked() {
                policy.density_percent = density;
            }
        }
    });
    ui.add(egui::Slider::new(&mut policy.density_percent, 0..=100).text("Context density %"));
    ui.small("100% is normal free-roam density. A larger budget allows more cars; longer distances place them farther away and can reduce nearby encounters.");
    ui.add(egui::Slider::new(&mut policy.spawn_min, 150..=1200).text("Minimum spawn distance · m"));
    ui.add(egui::Slider::new(&mut policy.spawn_max, (policy.spawn_min + 50)..=1400).text("Maximum spawn probe · m"));
    ui.add(
        egui::Slider::new(&mut policy.retire_distance, (policy.spawn_max + 200)..=2400)
            .text("Maximum unseen range · m"),
    );
    ui.add(egui::Slider::new(&mut policy.offscreen_seconds, 4..=30).text("Offscreen grace · s"));
    ui.small("Range and grace are visibility controls, not density controls. Maxing every slider is not a maximum-traffic preset.");
    *policy = policy.sanitized();
    if before != settings.traffic_policy.range_only() {
        settings.traffic_policy.custom = true;
    }
    ui.small("The budget includes the player, police and trailers. Neighbourhood model limits can keep the actual count lower.");
    ui.small("Density scales the context rather than replacing it; pursuits still reduce traffic. Race-specific density/route integration remains incomplete.");
    ui.small("The probe gains one second of projected speed. Final road placement must stay outside the minimum. Collision support loads separately from scenery and shadows.");
    ui.separator();
    ui.add(egui::Slider::new(&mut settings.cop_share.0, 0..=100).text("Patrol share %"));
    ui.checkbox(&mut settings.traffic_lights, "Obey traffic lights (custom)");
    ui.checkbox(&mut settings.traffic_avoidance, "Lane-change avoidance (custom)");
    ui.small("Changing population does not flush visible cars. Reduce density or budget to let traffic drain as it leaves view.");
}
