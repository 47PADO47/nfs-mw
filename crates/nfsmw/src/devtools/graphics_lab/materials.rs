//! Material and capture controls for the native renderer.
use super::*;

pub(super) fn show(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &mut Settings) {
    let defaults = profiles::defaults(settings);
    controls::checkbox(
        ui,
        &mut lab.options.enhanced_windows,
        "Enhanced building windows",
        defaults.appearance.lighting.enhanced_windows,
    )
    .on_hover_text("Shallow display recesses, soft reflections and selected interior lights at dusk.");
    controls::checkbox(
        ui,
        &mut lab.options.texture_variation,
        "Texture variation (experimental)",
        defaults.appearance.lighting.texture_variation,
    )
    .on_hover_text("Breaks up the repeating grid of tiled grass, hillside, cliff and concrete textures. Road strips, buildings and decals keep their authored look. One extra texture read on those surfaces.");
    let lighting = &defaults.appearance.lighting;
    controls::checkbox(ui, &mut lab.options.parallax, "Parallax relief (experimental)", lighting.parallax)
        .on_hover_text("Gives depth to normal-mapped surfaces whose texture mod stores a height in the normal map's alpha, such as cobbles. Other surfaces are unchanged. Shapes, shadows and reflections keep the flat surface.");
    ui.add_enabled_ui(lab.options.parallax, |ui| {
        controls::slider(ui, &mut lab.options.parallax_depth, 0.0..=15.0, "Relief depth (cm)", lighting.parallax_depth);
        ui.horizontal(|ui| {
            ui.add(egui::Slider::new(&mut lab.options.parallax_steps, 4..=64).text("Parallax samples"));
            controls::reset(ui, &mut lab.options.parallax_steps, lighting.parallax_steps, "Parallax samples");
        });
        controls::slider(
            ui,
            &mut lab.options.parallax_distance,
            5.0..=200.0,
            "Parallax fade distance (m)",
            lighting.parallax_distance,
        );
    });
    ui.add_enabled_ui(lab.options.shadow_projection.is_none(), |ui| {
        controls::checkbox(
            ui,
            &mut lab.options.world_specular,
            "Roughness-based world highlights",
            defaults.appearance.lighting.world_specular,
        )
        .on_hover_text("Uses material roughness for sunlight and lamp highlights on normal-mapped scenery.");
        ui.add_enabled_ui(lab.options.world_specular, |ui| {
            controls::checkbox(
                ui,
                &mut lab.options.world_specular_aa,
                "World highlight anti-aliasing",
                defaults.appearance.lighting.world_specular_aa,
            )
            .on_hover_text(
                "Softens highlights where the normal changes sharply within a pixel. Compare with DLSS on or off.",
            );
        });
    });
    ui.horizontal(|ui| {
        egui::ComboBox::from_label("Texture filtering").selected_text(settings.texture_filtering.label()).show_ui(
            ui,
            |ui| {
                for value in crate::settings::TextureFiltering::ALL {
                    ui.selectable_value(&mut settings.texture_filtering, value, value.label());
                }
            },
        );
        controls::reset(ui, &mut settings.texture_filtering, defaults.texture_filtering, "Texture filtering");
    });
    ui.small("Trilinear or 8×/16× anisotropic filtering, including normal-mapped roads. Quality presets keep 16×.");
    if lab.caps.is_some_and(|c| c.restart_required.contains(blackbox_gfx::Setting::TextureFiltering)) {
        ui.small("Texture filtering applies after restart. Save for startup to keep the selection.");
    }
    ui.small("Car materials and automatic lighting are configured in Lighting.");
    ui.collapsing("Advanced material paths", |ui| {
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Car material renderer").selected_text(settings.car_shading.label()).show_ui(ui, |ui| {
                for value in crate::settings::CarShading::ALL {
                    ui.selectable_value(&mut settings.car_shading, value, value.label());
                }
            });
            controls::reset(ui, &mut settings.car_shading, defaults.car_shading, "Car material renderer");
        });
        ui.small("Enhanced supports paint finishes, layered materials and environment lighting. Original PC is a reconstruction with remaining parity gaps. Simple reduces material detail.");
    });
    ui.add_enabled_ui(lab.caps.is_none_or(|c| c.local_reflections), |ui| {
        controls::choice(
            ui,
            "Car reflections",
            &mut settings.reflections,
            &ReflectionQuality::ALL,
            defaults.reflections,
        );
    });
    ui.small("Captures the car surroundings. Resolution controls detail; update rate controls motion smoothness.");
    ui.add_enabled_ui(lab.caps.is_some_and(|c| c.local_reflections), |ui| {
        controls::checkbox(ui, &mut lab.options.cached_reflections, "Cached car reflections (High: 60 Hz)", defaults.appearance.lighting.cached_reflections)
            .on_hover_text("Captures all six faces together at 60 Hz (High), 20 Hz (Medium), or 10 Hz (Low), limited by base FPS. One metre of movement or streamed scenery refreshes early. Off: High captures every base frame. No image blending.");
        controls::checkbox(ui, &mut lab.options.screen_reflections, "Selective screen-space car reflections", defaults.appearance.lighting.screen_reflections)
            .on_hover_text("Reflects visible current-frame scenery on car materials. Misses and screen edges use the local probe. Can add GPU cost; HDR and frame generation keep separate UI.");
    });
    ui.add_enabled_ui(lab.ray_reflections_available, |ui| {
        controls::checkbox(ui, &mut lab.options.ray_car_reflections, "Hardware RT car reflections", defaults.appearance.lighting.ray_car_reflections)
            .on_hover_text("Reflects loaded geometry on Enhanced car paint, glass, rims and chrome. Misses and rough finishes retain filtered cubemaps. Uses consistent approximate hit lighting, without screen-colour switching. Original PC materials retain their existing response.");
        egui::ComboBox::from_label("RT texture detail")
            .selected_text(["Low (~213 MiB)", "Balanced (~427 MiB)", "High (~1.17 GiB)", "Very High (~2.17 GiB)"][lab.options.ray_texture_quality.min(3) as usize])
            .show_ui(ui, |ui| {
                for (value, label) in ["Low", "Balanced", "High", "Very High"].into_iter().enumerate() {
                    ui.selectable_value(&mut lab.options.ray_texture_quality, value as u32, label);
                }
            }).response.on_hover_text("Shared by RT car and road reflections. Automatically gives nearby textures more detail within a fixed cache budget. Does not add rays. Changing this rebuilds the cache and can briefly pause rendering. Memory values cover RT colour textures only; source textures and RT geometry use additional VRAM.");
        ui.collapsing("Advanced RT car reflections", |ui| {
            ui.add(egui::Slider::new(&mut lab.options.ray_car_range, 40.0..=800.0).text("Car reflection ray range (m)"));
            ui.add(egui::Slider::new(&mut lab.options.ray_car_blend, 0.0..=1.0).text("Car RT geometry contribution"));
        });
        controls::checkbox(ui, &mut lab.options.car_rt_lamp_lighting, "Night-lit RT car reflections", lighting.car_rt_lamp_lighting)
            .on_hover_text("Lights what RT car reflections hit as the world is lit: dark at night, with the street lamps' pools of light and lit windows. Off: the older daylight hit lighting.");
    });
    ui.collapsing("Car paint at night", |ui| {
        controls::slider(ui, &mut lab.options.car_night_reflections, 0.25..=3.0, "Car reflections at night", lighting.car_night_reflections)
            .on_hover_text("Brightness of a clear coat's sharp reflection at night, relative to the night street it mirrors: 1 is as bright as the street, higher reads glossier. Chrome, bare metal and matte finishes take a milder lift (the square root). Daylight is unchanged.");
        controls::slider(ui, &mut lab.options.car_light_strength, 0.0..=5.0, "Car light reflections", lighting.car_light_strength)
            .on_hover_text("Brightness of lamp highlights and of bright reflected lights (lamps, signs, lit windows) in car paint at night. 1 is unboosted.");
        controls::slider(ui, &mut lab.options.car_lamp_size, 0.0..=1.0, "Lamp highlight size (m)", lighting.car_lamp_size)
            .on_hover_text("Size of the street lamps mirrored in car paint (headlights are half of it): their highlights grow as the car nears them. 0 treats each lamp as a point.");
    });
    super::reflections::show(ui, lab, settings);
    roads::show(ui, lab, settings);
}
