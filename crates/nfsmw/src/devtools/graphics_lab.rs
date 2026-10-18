//! F8 graphics lab: canonical previews, explicit persistence and reversible comparisons.
mod bevy;
mod controls;
mod detail;
mod display;
mod effects;
mod environment;
mod lighting;
mod materials;
mod presets;
mod profiles;
mod quality;
mod reflections;
mod roads;
mod saved;
mod traffic;
mod video;
mod view;
#[cfg(test)]
mod workflow_tests;

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
    options: RenderExperiments,
    startup_options: Option<RenderExperiments>,
    tab: profiles::Tab,
    save_requested: bool,
    disk: saved::Presets,
    profile_path: Option<std::path::PathBuf>,
    message: Option<String>,
    caps: Option<blackbox_gfx::Capabilities>,
    display: blackbox_gfx::DisplayStatus,
    latency: blackbox_gfx::LatencyStatus,
    startup_quality: Option<quality::Quality>,
    startup_style: Option<crate::settings::VisualStyle>,
    stats: ExperimentStats,
    active: bool,
    restore_capture: bool,
    original: Option<(RenderExperiments, Settings)>,
    saved: Option<(RenderExperiments, Settings)>,
    environment: Option<crate::viewer::Environment>,
    environment_change: Option<crate::viewer::Environment>,
    traffic_status: Option<String>,
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
            Ok("blockers") => LightingDebug::ShadowBlockers,
            Ok("penumbra") => LightingDebug::ShadowPenumbra,
            _ => LightingDebug::Off,
        };
        if std::env::var("NFSMW_LAB_WORLD_AMBIENT").as_deref() == Ok("0") {
            options.world_ambient = false;
        }
        Self {
            open: std::env::var("NFSMW_LAB_OVERLAY").as_deref() == Ok("1")
                || (!preset.is_empty() && std::env::var("NFSMW_LAB_OVERLAY").as_deref() != Ok("0")),
            startup_options: (!preset.is_empty()
                || std::env::var("NFSMW_LAB_FROZEN_SUN").is_ok()
                || std::env::var("NFSMW_LAB_DEBUG").is_ok())
            .then_some(options),
            tab: match std::env::var("NFSMW_LAB_TAB").as_deref() {
                Ok("video") => profiles::Tab::Video,
                Ok("lighting") => profiles::Tab::Lighting,
                Ok("materials") => profiles::Tab::Materials,
                Ok("detail") => profiles::Tab::Detail,
                Ok("traffic") => profiles::Tab::Traffic,
                Ok("effects") => profiles::Tab::Effects,
                Ok("environment") => profiles::Tab::Environment,
                Ok("diagnostics") => profiles::Tab::Diagnostics,
                _ => profiles::Tab::Look,
            },
            save_requested: false,
            disk: Default::default(),
            profile_path: Settings::config_path(),
            message: None,
            caps: None,
            display: Default::default(),
            latency: Default::default(),
            options,
            startup_style: std::env::var("NFSMW_LAB_STYLE").ok().and_then(|s| s.parse().ok()),
            startup_quality: std::env::var("NFSMW_LAB_QUALITY").ok().as_deref().and_then(quality::Quality::parse),
            stats: Default::default(),
            active: false,
            restore_capture: false,
            original: None,
            saved: None,
            environment: None,
            environment_change: None,
            traffic_status: None,
        }
    }
}

fn preset_options(name: &str) -> RenderExperiments {
    RenderExperiments {
        original_world_materials: true,
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
        .add_systems(
            PreUpdate,
            (quality::startup, toggle, presets::hotkeys)
                .chain()
                .after(InputSystems)
                .before(crate::input::update_actions),
        )
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
    mut garage: ResMut<crate::garage::Overlay>,
) {
    if keys.just_pressed(KeyCode::F8) {
        if console.open {
            console.set_open(false, &mut capture, &mut focus);
        }
        if garage.open {
            garage.set_open(false, &mut capture, &mut focus);
        }
        lab.open = !lab.open;
    }
    if keys.just_pressed(KeyCode::F12) || keys.just_pressed(KeyCode::F6) || garage.start_requested {
        lab.open = false;
    }
    if lab.open && keys.just_pressed(KeyCode::Escape) {
        lab.open = false;
        keys.reset(KeyCode::Escape);
    }
    if console.open {
        lab.open = false;
    }
    lab.sync_focus(&mut capture, &mut focus);
}

fn sync(mut lab: ResMut<GraphicsLab>, mut capture: ResMut<MouseCapture>, mut focus: ResMut<UiFocus>) {
    lab.sync_focus(&mut capture, &mut focus);
}

impl GraphicsLab {
    pub(crate) fn open_from_menu(&mut self, capture: &mut MouseCapture, focus: &mut UiFocus) {
        self.open = true;
        self.sync_focus(capture, focus);
    }

    pub(crate) fn close(&mut self, capture: &mut MouseCapture, focus: &mut UiFocus) {
        self.open = false;
        self.sync_focus(capture, focus);
    }

    fn sync_focus(&mut self, capture: &mut MouseCapture, focus: &mut UiFocus) {
        if self.open {
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

fn apply(mut lab: ResMut<GraphicsLab>, settings: Res<Settings>, mut host: NonSendMut<Host>) {
    let Some(renderer) = host.renderer.as_mut() else { return };
    renderer.set_render_experiments(settings.appearance.lighting);
    lab.caps = Some(*renderer.capabilities());
    lab.display = renderer.display_status();
    lab.latency = renderer.latency_status();
    lab.stats = renderer.experiment_stats();
}

pub fn show_world(
    ctx: &egui::Context,
    lab: &mut GraphicsLab,
    settings: &mut Settings,
    metrics: &super::metrics::Metrics,
    scene: &mut dyn crate::viewer::Scene,
) {
    lab.environment = scene.environment();
    lab.traffic_status = (lab.open && lab.tab == profiles::Tab::Traffic).then(|| scene.traffic_status()).flatten();
    show(ctx, lab, settings, metrics);
    if let Some(value) = lab.environment_change.take() {
        scene.set_environment(value);
    }
}

pub fn show(ctx: &egui::Context, lab: &mut GraphicsLab, settings: &mut Settings, metrics: &super::metrics::Metrics) {
    lab.options = settings.appearance.lighting;
    let old_shadow = lab.options.sun_shadows;
    view::show(ctx, lab, settings, metrics);
    settings.appearance.lighting = lab.options.sanitized();
    if old_shadow != lab.options.sun_shadows {
        settings.shadows = match (lab.options.sun_shadows, settings.appearance.quality) {
            (false, _) => crate::settings::ShadowQuality::Off,
            (true, crate::settings::GraphicsQuality::Low) => crate::settings::ShadowQuality::Low,
            (true, crate::settings::GraphicsQuality::Balanced) => crate::settings::ShadowQuality::Medium,
            (true, crate::settings::GraphicsQuality::High | crate::settings::GraphicsQuality::VeryHigh) => {
                crate::settings::ShadowQuality::High
            }
        };
    }
    if lab.save_requested {
        lab.save_requested = false;
        saved::save_startup(lab, settings);
    }
}

#[cfg(test)]
#[path = "graphics_lab_tests.rs"]
mod tests;
