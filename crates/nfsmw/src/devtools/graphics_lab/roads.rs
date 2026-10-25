//! Road capture controls, separate from car cubemap quality.
use super::*;
pub(super) fn show(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &mut Settings) {
    let supported = lab.caps.is_some_and(|caps| caps.projected_reflections);
    let receivers = lab.options.original_world_materials;
    ui.separator();
    ui.add_enabled_ui(supported && receivers, |ui| {
        let defaults = super::profiles::defaults(settings);
        super::controls::checkbox(ui, &mut lab.options.enhanced_wet_surfaces, "Enhanced wet surfaces", defaults.appearance.lighting.enhanced_wet_surfaces)
            .on_hover_text("Separates damp asphalt from pooled water on authored reflective surfaces. Patches remain fixed in the world and dry after rain. Uses hardware RT when available, otherwise screen-space reflections. The environment capture fills missing and distant detail.");
        ui.add_enabled_ui(lab.ray_reflections_available && lab.options.enhanced_wet_surfaces, |ui| {
            super::controls::checkbox(ui, &mut lab.options.ray_road_reflections, "Hardware RT road reflections", defaults.appearance.lighting.ray_road_reflections)
                .on_hover_text("Traces loaded geometry, including off-screen objects. Visible hits can reuse screen colour; misses use the environment capture. Off switches to screen-space road reflections immediately. Car paint keeps its separate reflection system.");
        });
        if !lab.ray_reflections_available {
            ui.small("Hardware RT is unavailable in this session. Use the Vulkan RT launcher to enable it at startup.");
        }
        ui.collapsing("Advanced wet surfaces", |ui| {
            ui.add(egui::Slider::new(&mut lab.options.puddle_coverage, 0.0..=1.0).text("Puddle coverage"));
            ui.add(egui::Slider::new(&mut lab.options.puddle_scale, 2.0..=30.0).text("Puddle patch size (m)"));
            ui.add(egui::Slider::new(&mut lab.options.water_ripples, 0.0..=2.0).text("Rain ripples"));
            ui.add(egui::Slider::new(&mut lab.options.road_reflection_range, 80.0..=800.0).text("Reflection ray range (m)"))
                .on_hover_text("Maximum distance along a reflected ray, not distance ahead of the car. Longer rays retain distant objects but cost more GPU time. Hardware RT can trace loaded off-screen geometry; screen-space mode uses the environment capture outside the view.");
            ui.add(egui::Slider::new(&mut lab.options.road_reflection_fade, 0.1..=0.9).text("Reflection distance blend"))
                .on_hover_text("Fraction of the ray range used for a gradual transition to the environment capture. Higher values soften the visible boundary but reduce distant reflection contrast. Defaults: 320 m range, 0.65 blend.");
            ui.add_enabled_ui(lab.ray_reflections_available && lab.options.ray_road_reflections, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut lab.options.ray_road_blend, 0.0..=1.0).text("RT geometry contribution"))
                        .on_hover_text("Blends reflected geometry with the environment capture. 1 keeps the full RT hit; 0 shows only the capture. Does not change material wetness or the range fade.");
                    super::controls::reset(ui, &mut lab.options.ray_road_blend, defaults.appearance.lighting.ray_road_blend, "RT geometry contribution");
                });
                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut lab.options.ray_road_screen_reuse, 0.0..=1.0).text("RT screen colour reuse"))
                        .on_hover_text("At an exact visible RT hit, blends approximate hit lighting with the fully shaded screen colour. 1 preserves decals and detailed shading; 0 uses the same approximate lighting as off-screen hits. Intersection remains hardware RT.");
                    super::controls::reset(ui, &mut lab.options.ray_road_screen_reuse, defaults.appearance.lighting.ray_road_screen_reuse, "RT screen colour reuse");
                });
            });
            ui.small("Procedural puddles approximate uneven roads; they do not simulate drainage or map tunnel shelter. Rain ripples stop after rainfall; water remains until it dries.");
        });
        if lab.options.enhanced_wet_surfaces {
            let mut enabled = settings.road_reflections != crate::settings::RoadReflections::Off;
            if ui.checkbox(&mut enabled, "Road reflections").changed() {
                settings.road_reflections = match enabled {
                    true => crate::settings::RoadReflections::Vehicles,
                    false => crate::settings::RoadReflections::Off,
                };
            }
            ui.small("Road reflections use the internal render resolution. Hardware RT traces loaded geometry; screen-space mode traces visible geometry. Both blend to the car environment capture. Original road capture settings apply with Enhanced wet surfaces off.");
            return;
        }
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Road reflection detail")
                .selected_text(settings.road_reflections.label())
                .show_ui(ui, |ui| {
                    for value in crate::settings::RoadReflections::ALL {
                        ui.selectable_value(&mut settings.road_reflections, value, value.label());
                    }
                });
            super::controls::reset(
                ui,
                &mut settings.road_reflections,
                defaults.road_reflections,
                "Road reflection detail",
            );
        });
        ui.add_enabled_ui(settings.road_reflections != crate::settings::RoadReflections::Off, |ui| {
            ui.horizontal(|ui| {
                egui::ComboBox::from_label("Road reflection resolution")
                    .selected_text(settings.road_reflection_resolution.label())
                    .show_ui(ui, |ui| {
                        for value in crate::settings::RoadReflectionResolution::ALL {
                            ui.selectable_value(&mut settings.road_reflection_resolution, value, value.label());
                        }
                    });
                super::controls::reset(
                    ui,
                    &mut settings.road_reflection_resolution,
                    defaults.road_reflection_resolution,
                    "Road reflection resolution",
                );
            });
        });
    });
    if !supported {
        ui.small("Projected road captures require the native renderer. Stored choices are retained.");
        return;
    }
    if !receivers {
        ui.small("The active material fallback cannot receive road reflections. Stored choices are retained.");
        return;
    }
    if lab.options.enhanced_wet_surfaces {
        return;
    }
    ui.small("Original authored road capture; higher resolutions retain smaller reflected objects and use bounds from the reflected meshes. Rain wets the road gradually and reflections remain while it dries. Environment offers a separate comparison override.");
}
