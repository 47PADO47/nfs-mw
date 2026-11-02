//! Custom and original lighting controls with material dependency explanations.
use super::view::choice;
use super::*;

mod car;
mod dependencies;
mod world;

pub(super) fn show(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &mut Settings) {
    let defaults = super::profiles::defaults(settings).appearance.lighting;
    ui.add_enabled_ui(lab.stats.supported, |ui| {
        car::show(ui, lab, settings);
        ui.separator();
        world::show(ui, lab, settings);
        ui.separator();
        super::controls::checkbox(ui, &mut lab.options.sun_shadows, "Sun shadows", defaults.sun_shadows);
        ui.add_enabled_ui(lab.ray_reflections_available, |ui| {
            super::controls::checkbox(ui, &mut lab.options.ray_sun_shadows, "Hardware RT sun shadows", defaults.ray_sun_shadows)
                .on_hover_text("Vulkan prototype: sun shadows near the camera, cascaded shadows farther away. Reuses RT geometry. Soft mode can use a short, two-frame history. Requires an RT-enabled launch.");
        });
        super::controls::checkbox(ui, &mut lab.options.original_shadows, "Original shadow overlays", defaults.original_shadows);
        ui.collapsing("Advanced RT sun shadows", |ui| {
            ui.add_enabled_ui(lab.options.ray_sun_shadows, |ui| {
                super::controls::checkbox(ui, &mut lab.options.ray_soft_shadows, "Soft RT sunlight", defaults.ray_soft_shadows)
                    .on_hover_text("Samples the sun disc: contact edges stay tight, elevated blockers cast broader edges. Surface-aware upsampling and optional two-frame stabilization. Lower mask resolution reduces ray cost.");
                if lab.options.ray_soft_shadows {
                    super::controls::slider(ui, &mut lab.options.sun_angle, 0.0..=2.0, "Sun radius °", defaults.sun_angle);
                    super::controls::choice(ui, "RT mask resolution %", &mut lab.options.ray_shadow_resolution, &[50, 100], defaults.ray_shadow_resolution);
                    ui.small("50% traces one quarter of the pixels and reconstructs full-resolution visibility using depth and normals. Fine unmatched surfaces retain cascades. 100% is the quality reference.");
                    super::controls::choice(ui, "Sun rays per pixel", &mut lab.options.ray_shadow_samples, &[2, 4, 8, 16], defaults.ray_shadow_samples);
                    super::controls::slider(ui, &mut lab.options.ray_shadow_filter, 0.0..=2.0, "RT spatial smoothing", defaults.ray_shadow_filter);
                    super::controls::checkbox(ui, &mut lab.options.ray_shadow_temporal, "Two-frame stabilization", defaults.ray_shadow_temporal)
                        .on_hover_text("Blends this frame with the previous unblended spatial estimate. Reprojects motion, rejects changed surfaces and clamps changed shadows. No older history.");
                    ui.small("2 rays costs less but can add grain or flicker. 4 rays is the normal target. Surface-aware smoothing removes sample grain; 0 disables it and stabilization. Stabilization blends only two unblended frames.");
                }
                super::controls::slider(ui, &mut lab.options.ray_shadow_range, 10.0..=240.0, "RT receiver range (m)", defaults.ray_shadow_range);
                super::controls::slider(ui, &mut lab.options.ray_shadow_length, 40.0..=800.0, "RT blocker reach (m)", defaults.ray_shadow_length);
                super::controls::slider(ui, &mut lab.options.ray_shadow_bias, 0.001..=0.05, "RT surface bias (m)", defaults.ray_shadow_bias);
                ui.small("The outer 20% blends to cascades. Blocker reach includes off-camera sunward geometry. Higher reach can cost more; bias trades self-shadow acne for detached edges. Sun radius also controls soft RT sunlight. PCSS affects cascades only. RT replaces screen-space contact detail while active; AO remains independent.");
            });
        });
        ui.collapsing("Advanced shadows", |ui| {
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
                {
                    super::controls::checkbox(ui, &mut lab.options.near_shadow_focus, "Tight near focus", defaults.near_shadow_focus)
                        .on_hover_text("Doubles nearest detail by halving its coverage. Works with wide detail: its higher resolution retains car detail over a wider near area. Overlapping cascades blend smoothly.");
                }
                super::controls::checkbox(ui, &mut lab.options.cascade_temporal, "Two-frame cascade stabilization", defaults.cascade_temporal)
                    .on_hover_text("Blends current and previous unblended visibility using motion, depth and normal checks. Reduces edge shimmer; rejects camera cuts and changed surfaces. Adds a screen-space resolve. Inactive with RT shadows.");
                super::controls::checkbox(ui, &mut lab.options.cached_shadows, "Cache unchanged shadow cascades", defaults.cached_shadows)
                    .on_hover_text("Caches static depth only when the exact cascade projection is unchanged. Moving casters redraw every frame. Most effective with a stationary camera and frozen sun; moving projections use the original path.");
                let mut reach = if lab.options.far_shadows { lab.options.far_shadow_scale } else { 1.0 };
                super::controls::slider(ui, &mut reach, 1.0..=5.0, "Distant range ×", if defaults.far_shadows { defaults.far_shadow_scale } else { 1.0 });
                if reach != if lab.options.far_shadows { lab.options.far_shadow_scale } else { 1.0 } {
                    lab.options.far_shadows = reach > 1.01;
                    lab.options.far_shadow_scale = reach;
                }
                ui.small(format!("Detailed coverage {:.0} m; total shadow coverage {:.0} m. Longer distance adds distant caster work, not near detail.", lab.options.shadow_distance, lab.options.shadow_distance * reach));
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
        local_lights(ui, lab, settings);
        ui.collapsing("Lighting diagnostics", |ui| {
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
            ui.small("Custom cascades only. Blockers: black = 0, white = 8. Filter radius: black = 0, white = 8 shadow texels. PCSS off keeps a fixed radius.");
        }
        });
    });
    if !lab.stats.supported {
        ui.colored_label(egui::Color32::YELLOW, "These lighting controls require native Vulkan or DX12.");
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
    ui.small("World lighting uses the authored material inputs. Custom shadows and contact detail remain independent.");
}

/// Local lights: fixtures, vehicle beams, light fog and their shadow maps. Its own Lighting section, not nested under
/// Advanced shadows, because most of it is lighting rather than shadow tuning.
fn local_lights(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &Settings) {
    let defaults = super::profiles::defaults(settings).appearance.lighting;
    ui.collapsing("Local lights (prototype)", |ui| {
        super::controls::checkbox(ui, &mut lab.options.local_lamps, "Tunnel fixtures", defaults.local_lamps)
            .on_hover_text("Recognized tunnel fixtures throughout loaded scenery. Lights fade near the active budget boundary.");
        super::controls::checkbox(ui, &mut lab.options.local_street_lights, "Street fixtures", defaults.local_street_lights)
            .on_hover_text("Adds night lights to recognized streetlight models. Other models are not yet covered.");
        super::controls::checkbox(ui, &mut lab.options.local_headlights, "Player headlight beams", defaults.local_headlights)
            .on_hover_text("Uses the car's headlight switch and native lamp markers. Headlights share the local shadow budget.");
        super::controls::checkbox(ui, &mut lab.options.local_tail_lights, "Player tail/brake light spill", defaults.local_tail_lights)
            .on_hover_text("Adds a small red light behind the player car. It follows the light switch, braking and lamp damage.");
        super::controls::checkbox(ui, &mut lab.options.local_traffic_headlights, "Traffic headlight beams", defaults.local_traffic_headlights)
            .on_hover_text("Nearby traffic uses its lamp markers and damage state. Beams share the light and shadow budgets.");
        super::controls::choice(ui, "Traffic cars with beams", &mut lab.options.local_traffic_limit, &[0, 2, 4, 6, 8], defaults.local_traffic_limit);
        super::controls::slider(ui, &mut lab.options.local_light_distance, 80.0..=600.0, "Local light visibility distance (m)", defaults.local_light_distance)
            .on_hover_text("Maximum camera distance for loaded tunnel and street fixtures. Higher values can increase lighting and shadow cost.");
        super::controls::slider(ui, &mut lab.options.local_light_strength, 0.0..=5.0, "Tunnel light strength", defaults.local_light_strength);
        super::controls::slider(ui, &mut lab.options.local_street_strength, 0.0..=5.0, "Street light strength", defaults.local_street_strength);
        super::controls::slider(ui, &mut lab.options.local_street_glow, 0.0..=2.0, "Street lamp glow", defaults.local_street_glow)
            .on_hover_text("Changes the visible lamp glow without changing illumination or shadow cost.");
        super::controls::slider(ui, &mut lab.options.local_light_fog, 0.0..=4.0, "Local light fog", defaults.local_light_fog)
            .on_hover_text("Weather-dependent illuminated air from nearby local lights. Zero disables it.");
        super::controls::slider(ui, &mut lab.options.local_light_fog_distance, 80.0..=600.0, "Fog visibility distance (m)", defaults.local_light_fog_distance)
            .on_hover_text("Distance over which local light illuminates fog. Fixture visibility and streaming also limit coverage.");
        super::controls::choice(ui, "Lights affecting fog", &mut lab.options.local_light_fog_count, &[0, 2, 4, 8, 16, 24, 32, 48, 64], defaults.local_light_fog_count);
        super::controls::choice(ui, "Fog grid quality", &mut lab.options.local_light_fog_quality, &[1, 2, 3], defaults.local_light_fog_quality);
        ui.small("Fog grid: 1 = low, 2 = medium, 3 = high. Higher quality and light counts can increase GPU cost. Loaded fixtures limit coverage.");
        super::controls::slider(ui, &mut lab.options.local_street_range, 10.0..=120.0, "Street light reach (m)", defaults.local_street_range);
        super::controls::slider(ui, &mut lab.options.local_headlight_strength, 0.0..=5.0, "Headlight strength", defaults.local_headlight_strength);
        super::controls::choice(ui, "Active light limit", &mut lab.options.local_light_limit, &[8, 16, 32, 64, 128], defaults.local_light_limit);
        super::controls::choice(ui, "Shadowed lights", &mut lab.options.local_shadow_count, &[0, 2, 4, 8, 16], defaults.local_shadow_count);
        super::controls::choice(ui, "Local shadow resolution", &mut lab.options.local_shadow_resolution, &[512, 1024, 2048], defaults.local_shadow_resolution);
        super::controls::slider(ui, &mut lab.options.local_shadow_filter, 0.0..=3.0, "Local shadow softness", defaults.local_shadow_filter);
        super::controls::slider(ui, &mut lab.options.local_shadow_distance, 20.0..=240.0, "Local shadow distance (m)", defaults.local_shadow_distance);
        super::controls::slider(ui, &mut lab.options.local_specular_strength, 0.0..=1.0, "Local highlight strength", defaults.local_specular_strength)
            .on_hover_text("Adds local specular highlights on reflective surfaces. Set to zero to skip their extra shading.");
        let mib = lab.options.local_shadow_count * lab.options.local_shadow_resolution.pow(2) * 4 / (1024 * 1024);
        ui.small(format!("Local shadow depth storage: {mib} MiB. All selected maps refresh each frame."));
        ui.small("0 shadowed lights keeps illumination. Headlights take priority; other nearby lamps share the remaining maps. RT above controls sun shadows, not these local maps.");
    });
}
