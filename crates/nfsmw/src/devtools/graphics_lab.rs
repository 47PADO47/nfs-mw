//! F8 graphics lab. All changes last for this session; the menu and config schema stay unchanged.
use crate::{
    app::{FrameSet, Host},
    input::{MouseCapture, UiFocus},
    settings::{PostAa, PostBloom, PostTonemap, Settings, VisualTreatment},
};
use bevy_app::{App, PreUpdate, Update};
use bevy_ecs::prelude::*;
use bevy_input::{ButtonInput, InputSystems, keyboard::KeyCode};
use blackbox_gfx::{AoQuality, ExperimentStats, LightingDebug, ReflectionQuality, RenderExperiments};

#[derive(Resource)]
pub struct GraphicsLab {
    pub open: bool,
    interacting: bool,
    options: RenderExperiments,
    stats: ExperimentStats,
    active: bool,
    restore_capture: bool,
    original: Option<Settings>,
    saved: Option<(RenderExperiments, Settings)>,
}

impl Default for GraphicsLab {
    fn default() -> Self {
        let preset = std::env::var("NFSMW_GRAPHICS_LAB").unwrap_or_default();
        let mut options = preset_options(&preset);
        options.freeze_sun = std::env::var("NFSMW_LAB_FROZEN_SUN").as_deref() == Ok("1");
        options.lighting_debug = match std::env::var("NFSMW_LAB_DEBUG").as_deref() {
            Ok("sun") => LightingDebug::SunVisibility,
            Ok("ao") => LightingDebug::AmbientVisibility,
            Ok("cascades") => LightingDebug::Cascades,
            _ => LightingDebug::Off,
        };
        if std::env::var("NFSMW_LAB_WORLD_AMBIENT").as_deref() == Ok("0") {
            options.world_ambient = false;
        }
        Self {
            open: !preset.is_empty() && std::env::var("NFSMW_LAB_OVERLAY").as_deref() != Ok("0"),
            options,
            interacting: false,
            stats: Default::default(),
            active: false,
            restore_capture: false,
            original: None,
            saved: None,
        }
    }
}

fn preset_options(name: &str) -> RenderExperiments {
    RenderExperiments {
        sun_shadows: matches!(name, "shadows" | "distance" | "detail" | "modern" | "max"),
        far_shadows: matches!(name, "distance" | "detail" | "modern" | "max"),
        contact_shadows: matches!(name, "detail" | "modern" | "max"),
        clearcoat: matches!(name, "modern" | "max"),
        car_skylight: matches!(name, "ambient" | "shadows" | "distance" | "detail" | "modern" | "max"),
        world_ambient: matches!(name, "ambient" | "shadows" | "distance" | "detail" | "modern" | "max"),
        original_shadows: !matches!(name, "shadows" | "distance" | "detail" | "modern" | "max"),
        shadow_resolution: match name {
            "max" => 4096,
            _ => 2048,
        },
        ..Default::default()
    }
}

pub fn build(app: &mut App) {
    app.init_resource::<GraphicsLab>()
        .add_systems(PreUpdate, toggle.after(InputSystems).before(crate::input::update_actions))
        .add_systems(
            Update,
            sync.in_set(FrameSet::Prepare)
                .after(crate::app::render::create_renderer)
                .after(crate::app::render::create_bevy_renderer)
                .before(crate::app::cursor_update),
        )
        .add_systems(Update, apply.in_set(FrameSet::SceneUpdate).before(crate::app::render::update_scene));
}

fn toggle(
    mut lab: ResMut<GraphicsLab>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut console: ResMut<super::Console>,
    mut capture: ResMut<MouseCapture>,
    mut focus: ResMut<UiFocus>,
) {
    if keys.just_pressed(KeyCode::F8) {
        if console.open {
            console.set_open(false, &mut capture, &mut focus);
        }
        lab.open = !lab.open;
    }
    if keys.just_pressed(KeyCode::F12) {
        lab.open = false;
    }
    lab.interacting = lab.open && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight));
    if lab.open && lab.interacting && keys.just_pressed(KeyCode::Escape) {
        lab.open = false;
        keys.reset(KeyCode::Escape);
    }
    lab.sync_focus(&mut capture, &mut focus);
}

fn sync(mut lab: ResMut<GraphicsLab>, mut capture: ResMut<MouseCapture>, mut focus: ResMut<UiFocus>) {
    lab.sync_focus(&mut capture, &mut focus);
}

impl GraphicsLab {
    fn sync_focus(&mut self, capture: &mut MouseCapture, focus: &mut UiFocus) {
        if self.open && self.interacting {
            if !self.active || capture.0 {
                // Renderer creation or a scene change can request capture while the lab is open.
                self.restore_capture = capture.0;
            }
            self.active = true;
            capture.0 = false;
            focus.0 = true;
            return;
        }
        if !self.active {
            return;
        }
        self.active = false;
        capture.0 = self.restore_capture;
        focus.0 = false;
    }
}

fn apply(mut lab: ResMut<GraphicsLab>, mut host: NonSendMut<Host>) {
    let Some(renderer) = host.renderer.as_mut() else { return };
    renderer.set_render_experiments(lab.options);
    lab.stats = renderer.experiment_stats();
}

pub fn show(ctx: &egui::Context, lab: &mut GraphicsLab, settings: &mut Settings, metrics: &super::metrics::Metrics) {
    lab.original.get_or_insert(*settings);
    if !lab.open {
        return;
    }
    if !lab.interacting {
        ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
    }
    egui::Window::new("Graphics lab · F8")
        .default_pos(egui::pos2(20.0, 80.0))
        .default_width(340.0)
        .default_height(620.0)
        .resizable(false)
        .interactable(lab.interacting)
        .vscroll(true)
        .open(&mut lab.open)
        .show(ctx, |ui| {
            ui.label(format!(
                "{:.0} FPS · {:.2} ms · worst {:.2} ms",
                metrics.fps(),
                metrics.average_ms(),
                metrics.worst_ms()
            ));
            ui.small("Changes apply live and are not saved to your config.");
            ui.small("Drive with this panel visible. Hold Alt to adjust options.");
            ui.horizontal(|ui| {
                if ui.button("Store A").clicked() {
                    lab.saved = Some((lab.options, *settings));
                }
                if ui.add_enabled(lab.saved.is_some(), egui::Button::new("Swap A/B")).clicked() {
                    let (options, snapshot) = lab.saved.replace((lab.options, *settings)).expect("enabled saved state");
                    lab.options = options;
                    restore_graphics(settings, &snapshot);
                }
                if ui.button("Restore startup").clicked() {
                    lab.options = RenderExperiments::default();
                    restore_graphics(settings, lab.original.as_ref().expect("initial settings"));
                }
            });
            ui.horizontal_wrapped(|ui| {
                for (name, preset) in [
                    ("Off", "baseline"),
                    ("Sky", "ambient"),
                    ("Sun", "shadows"),
                    ("Far", "distance"),
                    ("Detail", "detail"),
                    ("Coat", "modern"),
                ] {
                    if ui.button(name).clicked() {
                        let previous = lab.options;
                        lab.options = preset_options(preset);
                        lab.options.freeze_sun = previous.freeze_sun;
                        lab.options.lighting_debug = previous.lighting_debug;
                    }
                }
            });
            ui.add_enabled_ui(lab.stats.supported, |ui| {
                ui.checkbox(&mut lab.options.freeze_sun, "Freeze sun")
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
                    ],
                );
                if lab.options.lighting_debug != LightingDebug::Off {
                    ui.small("Debug bypasses colour effects. White means visible; black means occluded.");
                }
                ui.checkbox(&mut lab.options.sun_shadows, "Sun shadows");
                ui.add_enabled_ui(lab.options.sun_shadows, |ui| {
                    ui.checkbox(&mut lab.options.far_shadows, "Distant coverage (3× range)");
                    egui::ComboBox::from_label("Map resolution")
                        .selected_text(lab.options.shadow_resolution.to_string())
                        .show_ui(ui, |ui| {
                            for size in [512, 1024, 2048, 4096] {
                                ui.selectable_value(&mut lab.options.shadow_resolution, size, size.to_string());
                            }
                        });
                    ui.add(egui::Slider::new(&mut lab.options.shadow_distance, 40.0..=320.0).text("Near range"));
                    ui.add(egui::Slider::new(&mut lab.options.shadow_filter, 0.0..=3.0).text("Filter width"));
                    ui.checkbox(&mut lab.options.soft_shadows, "Contact-hardening sun (PCSS)");
                    if lab.options.soft_shadows {
                        ui.add(egui::Slider::new(&mut lab.options.sun_angle, 0.0..=2.0).text("Sun radius °"));
                    }
                });
                ui.checkbox(&mut lab.options.contact_shadows, "Contact shadow detail");
                if lab.options.contact_shadows {
                    ui.add(egui::Slider::new(&mut lab.options.contact_length, 0.05..=0.8).text("Contact reach"));
                }
                ui.checkbox(&mut lab.options.clearcoat, "Car clearcoat experiment");
                ui.checkbox(&mut lab.options.car_skylight, "Car sky and ground light");
                if lab.options.car_skylight {
                    ui.add(egui::Slider::new(&mut lab.options.sky_strength, 0.0..=2.0).text("Sky strength"));
                    ui.add(egui::Slider::new(&mut lab.options.ground_strength, 0.0..=2.0).text("Ground bounce"));
                }
                ui.checkbox(&mut lab.options.world_ambient, "World ambient lighting");
                if lab.options.world_ambient {
                    ui.add(egui::Slider::new(&mut lab.options.world_ambient_share, 0.35..=0.9).text("Shade fill"));
                    ui.add(egui::Slider::new(&mut lab.options.world_ambient_tint, 0.0..=1.0).text("Ambient colour"));
                }
            });
            ui.checkbox(&mut lab.options.original_shadows, "Original shadow overlays");
            if !lab.stats.supported {
                ui.colored_label(egui::Color32::YELLOW, "Prototypes require native Vulkan or DX12.");
            }
            ui.small(format!(
                "{} cascades · {} draws · {:.2} ms CPU setup · {} MiB atlas",
                lab.stats.cascades, lab.stats.shadow_draws, lab.stats.shadow_cpu_ms, lab.stats.map_mib
            ));
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
            ui.separator();
            choice(
                ui,
                "Car reflections",
                &mut settings.reflections,
                &[ReflectionQuality::Off, ReflectionQuality::Low, ReflectionQuality::Medium, ReflectionQuality::High],
            );
            choice(
                ui,
                "Ambient occlusion",
                &mut settings.ao,
                &[AoQuality::Off, AoQuality::Low, AoQuality::Medium, AoQuality::High],
            );
            choice(ui, "Tone mapping", &mut settings.post_tonemap, &[PostTonemap::Off, PostTonemap::Aces]);
            ui.small("AO affects ambient light; sunlight and reflections stay intact.");
            ui.checkbox(&mut settings.ao_compare, "AO split comparison");
            ui.add(egui::Slider::new(&mut settings.ao_intensity.0, 0..=100).text("AO intensity %"));
            choice(
                ui,
                "Bloom",
                &mut settings.post_bloom,
                &[PostBloom::Off, PostBloom::Low, PostBloom::Medium, PostBloom::High],
            );
            choice(ui, "AA", &mut settings.post_aa, &[PostAa::Off, PostAa::Fxaa, PostAa::Smaa]);
            choice(
                ui,
                "Visual Treatment",
                &mut settings.visual_treatment,
                &[VisualTreatment::Off, VisualTreatment::Low, VisualTreatment::High],
            );
            ui.small("F8 hides the panel. Alt+Esc closes it while adjusting. CPU setup is not GPU time.");
        });
}

fn choice<T: Copy + PartialEq + std::fmt::Display>(ui: &mut egui::Ui, label: &str, value: &mut T, choices: &[T]) {
    egui::ComboBox::from_label(label).selected_text(value.to_string()).show_ui(ui, |ui| {
        for choice in choices {
            ui.selectable_value(value, *choice, choice.to_string());
        }
    });
}

fn restore_graphics(settings: &mut Settings, snapshot: &Settings) {
    settings.reflections = snapshot.reflections;
    settings.ao = snapshot.ao;
    settings.ao_intensity = snapshot.ao_intensity;
    settings.ao_compare = snapshot.ao_compare;
    settings.post_tonemap = snapshot.post_tonemap;
    settings.post_bloom = snapshot.post_bloom;
    settings.post_aa = snapshot.post_aa;
    settings.visual_treatment = snapshot.visual_treatment;
}

#[cfg(test)]
#[path = "graphics_lab_tests.rs"]
mod tests;
