//! Bevy exposes its own supported settings, retaining hidden Black Box requests.
use super::view::choice;
use super::*;

pub(super) fn show(ui: &mut egui::Ui, lab: &GraphicsLab, settings: &mut Settings) {
    let Some(caps) = lab.caps else {
        ui.label("Waiting for the Bevy renderer…");
        return;
    };
    match lab.tab {
        profiles::Tab::Look => {
            let aa: Vec<_> = [
                (PostAa::Off, blackbox_gfx::Antialiasing::Off),
                (PostAa::Fxaa, blackbox_gfx::Antialiasing::Fxaa),
                (PostAa::Smaa, blackbox_gfx::Antialiasing::Smaa),
                (PostAa::Taa, blackbox_gfx::Antialiasing::Taa),
            ]
            .into_iter()
            .filter(|(_, aa)| caps.antialiasing.contains(*aa))
            .map(|(aa, _)| aa)
            .collect();
            choice(ui, "AA", &mut settings.post_aa, &aa);
            if caps.bloom {
                choice(
                    ui,
                    "Bloom",
                    &mut settings.post_bloom,
                    &[PostBloom::Off, PostBloom::Low, PostBloom::Medium, PostBloom::High],
                );
            }
            let tone: Vec<_> =
                [(PostTonemap::Off, blackbox_gfx::Tonemap::Off), (PostTonemap::Aces, blackbox_gfx::Tonemap::Aces)]
                    .into_iter()
                    .filter(|(_, tone)| caps.tonemaps.contains(*tone))
                    .map(|(tone, _)| tone)
                    .collect();
            choice(ui, "Tone mapping", &mut settings.post_tonemap, &tone);
        }
        profiles::Tab::Lighting => {
            choice(ui, "Sun shadows", &mut settings.shadows, &crate::settings::ShadowQuality::ALL);
            if caps.ray_tracing.is_available() {
                use crate::settings::RayTracingLevel as Rt;
                choice(ui, "Ray tracing", &mut settings.ray_tracing, &[Rt::Off, Rt::Low, Rt::Medium, Rt::High]);
                ui.small("Ray tracing changes take effect after restart.");
            }
        }
        profiles::Tab::Materials => {
            choice(ui, "Texture filtering", &mut settings.texture_filtering, &crate::settings::TextureFiltering::ALL);
            if caps.local_reflections {
                choice(ui, "Reflections", &mut settings.reflections, &ReflectionQuality::ALL);
            }
        }
        profiles::Tab::Effects if caps.ambient_occlusion => {
            choice(ui, "Ambient occlusion", &mut settings.ao, &AoQuality::ALL);
        }
        _ => {}
    }
}

pub(super) fn reset(tab: profiles::Tab, settings: &mut Settings) {
    let defaults = profiles::defaults(settings);
    match tab {
        profiles::Tab::Look => {
            settings.post_aa = defaults.post_aa;
            settings.post_bloom = defaults.post_bloom;
            settings.post_tonemap = defaults.post_tonemap;
        }
        profiles::Tab::Lighting => settings.shadows = defaults.shadows,
        profiles::Tab::Materials => {
            settings.texture_filtering = defaults.texture_filtering;
            settings.reflections = defaults.reflections;
        }
        profiles::Tab::Effects => settings.ao = defaults.ao,
        _ => {}
    }
}
