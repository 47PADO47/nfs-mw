//! Custom and original lighting controls with material dependency explanations.
use super::view::choice;
use super::*;

mod dependencies;

pub(super) fn show(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &Settings) {
    let defaults = super::profiles::defaults(settings).appearance.lighting;
    ui.add_enabled_ui(lab.stats.supported, |ui| {
        super::controls::checkbox(ui, &mut lab.options.freeze_sun, "Freeze sun", defaults.freeze_sun)
            .on_hover_text("Keeps the lighting phase fixed while you drive.");
        choice(
            ui,
            "Surface debug",
            &mut lab.options.lighting_debug,
            &[
                LightingDebug::Off,
                LightingDebug::SunVisibility,
                LightingDebug::AmbientVisibility,
                LightingDebug::Cascades,
                LightingDebug::ShadowBlockers,
                LightingDebug::ShadowPenumbra,
            ],
        );
        if matches!(lab.options.lighting_debug, LightingDebug::SunVisibility | LightingDebug::AmbientVisibility) {
            ui.small("Debug bypasses colour effects. White means visible; black means occluded.");
        }
        if matches!(lab.options.lighting_debug, LightingDebug::ShadowBlockers | LightingDebug::ShadowPenumbra) {
            ui.small("Custom cascades only. Blockers: black = 0, white = 16. Filter radius: black = 0, white = 8 shadow texels. PCSS off keeps a fixed radius.");
        }
        super::controls::checkbox(ui, &mut lab.options.sun_shadows, "Sun shadows", defaults.sun_shadows);
        super::controls::checkbox(ui, &mut lab.options.original_shadows, "Original shadow overlays", defaults.original_shadows);
        ui.add_enabled_ui(lab.options.sun_shadows, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Shadow system:");
                if ui.selectable_label(lab.options.shadow_projection.is_some(), "Original PC (partial)").clicked() {
                    dependencies::select_shadow_system(&mut lab.options, true);
                }
                if ui.selectable_label(lab.options.shadow_projection.is_none(), "Custom cascades").clicked() {
                    dependencies::select_shadow_system(&mut lab.options, false);
                }
            });
            if lab.options.shadow_projection.is_some() {
                ui.small("Single fitted perspective map; original camera and sun/cloud strength. Caster selection and vendor filtering remain incomplete. Stock size: 1024; larger maps need no new assets.");
            }
            super::controls::choice(ui, "Map resolution", &mut lab.options.shadow_resolution, &[512, 1024, 2048, 4096, 8192], defaults.shadow_resolution);
            ui.add_enabled_ui(lab.caps.is_some_and(|caps| caps.separate_shadow_casters), |ui| {
                super::controls::slider(ui, &mut lab.options.shadow_caster_min_radius, 0.0..=5.0,
                    "Minimum scenery caster radius (m)", defaults.shadow_caster_min_radius);
                ui.small("0 keeps all scenery shadows. Higher values omit small static casters; vehicles and dynamic props remain.");
            });
            ui.add_enabled_ui(lab.options.shadow_projection.is_none(), |ui| {
                super::controls::checkbox(ui, &mut lab.options.wide_shadow_detail, "Wide-detail cascades", defaults.wide_shadow_detail)
                    .on_hover_text("Keeps the first two cascades detailed over a wider area, with separate map resolution and consistent filter softness. Overrides legacy tight focus. Higher resolutions cost GPU time and memory.");
                if lab.options.wide_shadow_detail {
                    lab.options.shadow_detail_resolution = lab.options.shadow_detail_resolution.max(lab.options.shadow_resolution);
                    let sizes: Vec<_> = [512, 1024, 2048, 4096, 8192].into_iter().filter(|size| *size >= lab.options.shadow_resolution).collect();
                    super::controls::choice(ui, "Detail map resolution (first two)", &mut lab.options.shadow_detail_resolution, &sizes, defaults.shadow_detail_resolution.max(lab.options.shadow_resolution));
                    ui.small("Map resolution above controls distant cascades. Filter width uses the nearest map's texel size.");
                }
                if !lab.options.wide_shadow_detail {
                    super::controls::checkbox(ui, &mut lab.options.near_shadow_focus, "Tight near focus (legacy)", defaults.near_shadow_focus)
                        .on_hover_text("Doubles nearest detail by halving coverage; makes the quality transition appear closer to the camera.");
                }
                super::controls::checkbox(ui, &mut lab.options.cached_shadows, "Cache unchanged shadow cascades", defaults.cached_shadows)
                    .on_hover_text("Caches static depth only when the exact cascade projection is unchanged. Moving casters redraw every frame. Most effective with a stationary camera and frozen sun; moving projections use the original path.");
                super::controls::checkbox(ui, &mut lab.options.far_shadows, "Distant coverage (3× range)", defaults.far_shadows);
                super::controls::slider(ui, &mut lab.options.shadow_distance, 40.0..=320.0, "Near range", defaults.shadow_distance);
                super::controls::slider(ui, &mut lab.options.shadow_filter, 0.0..=3.0, "Filter width", defaults.shadow_filter);
                super::controls::checkbox(ui, &mut lab.options.soft_shadows, "Contact-hardening sun (PCSS)", defaults.soft_shadows);
                super::controls::checkbox(ui, &mut lab.options.dense_shadow_filter, "Dense penumbra filter", defaults.dense_shadow_filter)
                    .on_hover_text("Smooths gaps in wide shadow edges. Can increase GPU cost; narrow filters stay unchanged.");
                if lab.options.soft_shadows {
                    super::controls::slider(ui, &mut lab.options.sun_angle, 0.0..=2.0, "Sun radius °", defaults.sun_angle);
                }
            });
        });
        advanced(ui, lab, settings);
    });
    if !lab.stats.supported {
        ui.colored_label(egui::Color32::YELLOW, "Prototypes require native Vulkan or DX12.");
    }
    ui.small(format!(
        "{} cascades · {} draws · {:.2} ms CPU setup · {} MiB atlas",
        lab.stats.cascades, lab.stats.shadow_draws, lab.stats.shadow_cpu_ms, lab.stats.map_mib
    ));
    if lab.options.cached_shadows {
        ui.small(format!("{} cascades reused static depth this frame", lab.stats.shadow_cached_cascades));
    }
    if let Some(frame) = lab.stats.gpu_frame_ms {
        ui.small(format!(
            "GPU main {:.2} ms · sun {:.2} · geometry/contact {:.2} · AO {:.2}",
            frame,
            lab.stats.gpu_shadow_ms.unwrap_or(0.0),
            lab.stats.gpu_contact_ms.unwrap_or(0.0),
            lab.stats.gpu_ao_ms.unwrap_or(0.0)
        ));
    }
    if lab.stats.gpu_frame_ms.is_none() {
        ui.small("GPU timestamps unavailable or awaiting a sample");
    }
}

fn advanced(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &Settings) {
    let defaults = super::profiles::defaults(settings).appearance.lighting;
    let dependencies = dependencies::for_selection(
        settings.car_shading,
        lab.options.original_world_materials,
        lab.options.sun_shadows && lab.options.shadow_projection.is_some(),
    );
    super::controls::checkbox(ui, &mut lab.options.contact_shadows, "Contact shadow detail", defaults.contact_shadows)
        .on_hover_text("Custom screen-space occlusion for direct-lit world surfaces; it also works without a sun map.");
    ui.add_enabled_ui(lab.options.contact_shadows, |ui| {
        super::controls::slider(
            ui,
            &mut lab.options.contact_length,
            0.05..=0.8,
            "Contact reach",
            defaults.contact_length,
        );
    });
    if !lab.options.contact_shadows {
        ui.small("Enable Contact shadow detail to adjust its reach.");
    }
    match dependencies.contact_car {
        dependencies::ContactCar::ProjectedBypass => {
            ui.small("Contact detail can affect the world here. Original PC cars bypass the contact mask while the projected map is active.");
        }
        dependencies::ContactCar::ShadowlessBypass => {
            ui.small("Contact detail can affect the world here. Original PC (shadowless) cars do not receive it.");
        }
        dependencies::ContactCar::Direct => {
            ui.small("Contact detail affects visible direct-lit receivers; flat, bright or offscreen geometry may show little change.");
        }
    }
    ui.add_enabled_ui(dependencies.custom_car, |ui| {
        ui.add_enabled_ui(settings.renderer == crate::settings::RendererKind::Blackbox, |ui| {
            super::controls::checkbox(
                ui,
                &mut lab.options.paint_response,
                "Car material response",
                defaults.paint_response,
            );
        });
        super::controls::checkbox(ui, &mut lab.options.clearcoat, "Car clearcoat experiment", defaults.clearcoat);
        super::controls::checkbox(ui, &mut lab.options.car_skylight, "Car sky and ground light", defaults.car_skylight);
        ui.add_enabled_ui(lab.options.car_skylight, |ui| {
            super::controls::slider(
                ui,
                &mut lab.options.sky_strength,
                0.0..=2.0,
                "Sky strength",
                defaults.sky_strength,
            );
            super::controls::slider(
                ui,
                &mut lab.options.ground_strength,
                0.0..=2.0,
                "Ground bounce",
                defaults.ground_strength,
            );
        });
    });
    if !dependencies.custom_car {
        ui.small("Car clearcoat and sky/ground light require Custom Glossy in Materials. Original and Simple car shaders bypass these terms; stored values are retained.");
    }
    if dependencies.custom_car && !lab.options.car_skylight {
        ui.small("Enable Car sky and ground light to adjust sky strength and ground bounce.");
    }
    ui.small("World lighting uses the authored material inputs. Custom shadows and contact detail remain independent.");
}
