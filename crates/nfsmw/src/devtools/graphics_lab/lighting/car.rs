//! Player-facing car lighting with independent diagnostic overrides.
use super::super::{GraphicsLab, Settings, controls};
use crate::settings::CarShading;
use blackbox_gfx::RenderExperiments;

fn enhanced(options: &RenderExperiments) -> bool {
    options.paint_response
        && options.clearcoat
        && options.car_skylight
        && options.weather_car_lighting
        && options.local_reflection_correction
}

fn select(settings: &mut Settings, options: &mut RenderExperiments, original: bool) {
    if original {
        settings.car_shading = CarShading::Original;
        return;
    }
    settings.car_shading = CarShading::Glossy;
    options.paint_response = true;
    options.clearcoat = true;
    options.car_skylight = true;
    options.weather_car_lighting = true;
    options.local_reflection_correction = true;
}

pub(super) fn show(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &mut Settings) {
    let defaults = super::super::profiles::defaults(settings).appearance.lighting;
    let custom = super::dependencies::for_selection(
        settings.car_shading,
        lab.options.original_world_materials,
        lab.options.shadow_projection.is_some(),
    )
    .custom_car;
    ui.strong("Car materials and lighting");
    ui.horizontal_wrapped(|ui| {
        ui.label("Car lighting:");
        if ui
            .selectable_label(
                settings.car_shading == CarShading::Glossy && enhanced(&lab.options),
                "Enhanced (automatic)",
            )
            .clicked()
        {
            select(settings, &mut lab.options, false);
        }
        if ui.selectable_label(settings.car_shading == CarShading::Original, "Original PC").clicked() {
            select(settings, &mut lab.options, true);
        }
    });
    if custom {
        ui.small("Paint finish controls reflections and clearcoat. Sunlight follows weather; indirect light follows the captured sky, scenery and shelter.");
        if !enhanced(&lab.options) {
            ui.small("Advanced overrides are active. Select Enhanced (automatic) to restore the coordinated lighting features.");
        }
        ui.add_enabled_ui(lab.options.car_skylight, |ui| {
            controls::slider(ui, &mut lab.options.indirect_strength, 0.0..=2.0,
                "Indirect light strength", 1.0)
                .on_hover_text("1.0 keeps the calibrated sky/ground balance. Changes diffuse fill only; direct sunlight and reflections are unaffected.");
        });
    }
    if settings.car_shading.is_original() {
        ui.small("Reconstructed PC materials and light rig. Original rendering still has parity gaps. Enhanced tuning stays saved for switching back.");
    }
    ui.collapsing("Advanced car lighting", |ui| {
        ui.add_enabled_ui(custom, |ui| {
            controls::checkbox(ui, &mut lab.options.paint_response, "Finish and material response", defaults.paint_response)
                .on_hover_text("Uses each paint, metal, glass and carbon material's highlight width and reflection response.");
            controls::checkbox(ui, &mut lab.options.clearcoat, "Material clearcoat", defaults.clearcoat)
                .on_hover_text("Uses the coat assigned to each material. Matte gets a broad, weak coat; glass does not receive a second coat.");
            controls::checkbox(ui, &mut lab.options.local_reflection_correction, "Nearby reflection placement", defaults.local_reflection_correction)
                .on_hover_text("Uses current capture depth to improve nearby reflection placement. Hidden geometry and distant surfaces retain the probe fallback.");
            controls::checkbox(ui, &mut lab.options.weather_car_lighting, "Weather-matched sunlight", defaults.weather_car_lighting)
                .on_hover_text("Matches direct car light to the world's sun and cloud lighting. Does not dim environment light a second time.");
            controls::checkbox(ui, &mut lab.options.car_skylight, "Environment indirect light", defaults.car_skylight)
                .on_hover_text("Replaces fixed fill lights with broad lighting from the local environment capture, including roofs and nearby scenery.");
            ui.small("Traffic retains authored diffuse lighting with shared environment fill. Paint-only finish tuning keeps glass, trim and cargo separate.");
            ui.add_enabled_ui(lab.options.car_skylight, |ui| {
                controls::slider(ui, &mut lab.options.sky_strength, 0.0..=2.0, "Upper environment / sky", defaults.sky_strength);
                controls::slider(ui, &mut lab.options.ground_strength, 0.0..=2.0, "Lower environment / ground", defaults.ground_strength);
                ui.small("These are calibration multipliers. Captured brightness and colour already change with weather, time and nearby surfaces.");
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn switching_material_modes_keeps_tuning_and_unrelated_quality() {
        let mut settings = Settings::from(crate::settings::Partial::default());
        let mut options = RenderExperiments {
            sky_strength: 0.85,
            ground_strength: 0.45,
            indirect_strength: 1.2,
            cached_reflections: false,
            ..Default::default()
        };
        select(&mut settings, &mut options, false);
        assert!(enhanced(&options));
        assert_eq!(settings.car_shading, CarShading::Glossy);
        select(&mut settings, &mut options, true);
        assert_eq!(settings.car_shading, CarShading::Original);
        assert_eq!((options.sky_strength, options.ground_strength, options.indirect_strength), (0.85, 0.45, 1.2));
        assert!(!options.cached_reflections);
        options.clearcoat = false;
        assert!(!enhanced(&options));
    }
}
