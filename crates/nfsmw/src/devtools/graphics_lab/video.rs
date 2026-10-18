//! Window, pacing and reconstruction controls; renderer selection stays in Video options.
use super::{GraphicsLab, view::choice};
use crate::settings::{Monitor, RenderScale, Resolution, Settings, UpscaleMode, UpscaleQuality, WindowMode};

pub(super) fn show(ui: &mut egui::Ui, lab: &GraphicsLab, settings: &mut Settings) {
    if super::view::blackbox(lab, settings) {
        choice(ui, "Graphics API (after restart)", &mut settings.backend, &blackbox_gfx::GraphicsApi::ALL);
        ui.small("Save for startup and restart to change API. DLSS requires Vulkan.");
    }
    choice(
        ui,
        "Window mode",
        &mut settings.window_mode,
        &[WindowMode::Windowed, WindowMode::Borderless, WindowMode::Exclusive],
    );
    let mut monitors = vec![Monitor::Current, Monitor::Primary];
    if let Monitor::Index(index) = settings.monitor {
        monitors.push(Monitor::Index(index));
    }
    choice(ui, "Monitor", &mut settings.monitor, &monitors);
    let mut indexed = matches!(settings.monitor, Monitor::Index(_));
    if ui.checkbox(&mut indexed, "Choose monitor by index").changed() {
        settings.monitor = match indexed {
            true => Monitor::Index(0),
            false => Monitor::Current,
        };
    }
    if let Monitor::Index(index) = &mut settings.monitor {
        ui.horizontal(|ui| {
            ui.label("Monitor index (zero-based)");
            ui.add(egui::DragValue::new(index).range(0..=31));
        });
    }
    let sizes = [
        (1280, 720),
        (1600, 900),
        (1920, 1080),
        (1920, 1200),
        (2560, 1440),
        (2560, 1600),
        (3440, 1440),
        (3840, 2160),
        (5120, 1440),
        (5120, 2160),
        (7680, 4320),
    ];
    let mut resolutions = vec![Resolution::Native];
    resolutions.extend(sizes.map(|(width, height)| Resolution::Pixels { width, height }));
    if !resolutions.contains(&settings.resolution) {
        resolutions.push(settings.resolution);
    }
    choice(ui, "Resolution", &mut settings.resolution, &resolutions);
    if let Resolution::Pixels { width, height } = &mut settings.resolution {
        ui.horizontal(|ui| {
            ui.label("Custom size");
            ui.add(egui::DragValue::new(width).range(160..=16384).prefix("W "));
            ui.add(egui::DragValue::new(height).range(160..=16384).prefix("H "));
        });
    }
    ui.small("Borderless uses the desktop size. Exclusive mode needs a supported monitor mode; unavailable choices use the existing window fallback.");
    ui.checkbox(&mut settings.vsync, "VSync");
    ui.add_enabled_ui(!settings.vsync, |ui| {
        let mut limits: Vec<_> = ["unlocked", "30", "60", "90", "120", "144", "165", "240", "360"]
            .map(|v| v.parse().expect("frame limit"))
            .into();
        if !limits.contains(&settings.max_fps) {
            limits.push(settings.max_fps);
        }
        choice(ui, "Frame limit", &mut settings.max_fps, &limits);
    });
    choice(
        ui,
        "Performance overlay",
        &mut settings.show_metrics,
        &[
            crate::devtools::ShowMetrics::Off,
            crate::devtools::ShowMetrics::Basic,
            crate::devtools::ShowMetrics::Advanced,
        ],
    );
    ui.separator();
    ui.add_enabled_ui(lab.latency.reflex_available, |ui| {
        choice(ui, "NVIDIA Reflex", &mut settings.reflex, &crate::settings::ReflexMode::ALL);
    });
    let modes: Vec<_> = crate::settings::FrameGeneration::ALL
        .into_iter()
        .filter(|mode| mode.0 <= lab.latency.max_generated_frames)
        .collect();
    ui.add_enabled_ui(lab.latency.max_generated_frames > 0 && !settings.vsync && !lab.display.active, |ui| {
        choice(ui, "Frame generation", &mut settings.frame_generation, &modes);
    });
    ui.small("Frame generation requires HDR off, VSync off and DLSS/FSR 3. Reflex On is enforced while generation is requested. Native DLSS quality uses DLAA.");
    ui.small(lab.latency.note);
    if lab.latency.generation_active {
        ui.small(format!("Frame generation active; last SDK sample: {} frames actually presented. FPS above measures rendered frames.", lab.latency.presented_frames));
    }
    let offered = upscalers(lab.caps.as_ref());
    choice(ui, "Upscaler", &mut settings.upscaler, &offered);
    let effective = lab.caps.map(|c| settings.effective(&c)).unwrap_or(*settings);
    ui.add_enabled_ui(settings.upscaler.upscaler().is_temporal(), |ui| {
        choice(ui, "Upscale quality", &mut settings.upscale_quality, &UpscaleQuality::ALL);
    });
    if settings.upscaler == UpscaleMode::Dlss {
        ui.small("Native quality selects DLAA at output resolution.");
    }
    ui.add_enabled_ui(matches!(settings.upscaler, UpscaleMode::Bilinear | UpscaleMode::Fsr1), |ui| {
        let mut scale = settings.render_scale.percent();
        if ui.add(egui::Slider::new(&mut scale, 50..=200).text("Render scale %")).changed() {
            settings.render_scale = RenderScale::new(scale).expect("bounded scale");
        }
    });
    ui.add_enabled_ui(matches!(effective.upscaler, UpscaleMode::Fsr1 | UpscaleMode::Fsr3), |ui| {
        ui.add(egui::Slider::new(&mut settings.upscale_sharpness.0, 0..=100).text("Sharpening %"));
    });
    if !offered.contains(&settings.upscaler) {
        ui.colored_label(
            egui::Color32::YELLOW,
            format!("Requested {} is unavailable; running {}.", settings.upscaler, effective.upscaler),
        );
    }
    ui.small(
        "Temporal upscalers replace post-process AA. Unsupported stored requests remain available to save or replace.",
    );
}

pub(super) fn upscalers(caps: Option<&blackbox_gfx::Capabilities>) -> Vec<UpscaleMode> {
    [
        UpscaleMode::Off,
        UpscaleMode::Bilinear,
        UpscaleMode::Fsr1,
        UpscaleMode::Fsr3,
        UpscaleMode::Fsr4,
        UpscaleMode::Dlss,
    ]
    .into_iter()
    .filter(|mode| *mode == UpscaleMode::Off || caps.is_some_and(|c| c.upscalers.contains(mode.upscaler())))
    .collect()
}

pub(super) fn reset(settings: &mut Settings) {
    let defaults = Settings::from(crate::settings::Partial::default());
    settings.window_mode = defaults.window_mode;
    settings.monitor = defaults.monitor;
    settings.resolution = defaults.resolution;
    settings.vsync = defaults.vsync;
    settings.max_fps = defaults.max_fps;
    settings.reflex = defaults.reflex;
    settings.frame_generation = defaults.frame_generation;
    settings.show_metrics = defaults.show_metrics;
    settings.upscaler = defaults.upscaler;
    settings.upscale_quality = defaults.upscale_quality;
    settings.render_scale = defaults.render_scale;
    settings.upscale_sharpness = defaults.upscale_sharpness;
}
