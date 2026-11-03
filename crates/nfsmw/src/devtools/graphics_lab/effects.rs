//! Particle and camera effects are independent of visual style and quality presets.
use super::view::choice;
use crate::settings::{Settings, SmokeQuality, SparkStyle, SpeedTrailStyle};

pub(super) fn show(ui: &mut egui::Ui, settings: &mut Settings, preview: &mut Option<crate::settings::TailPreview>) {
    ui.separator();
    ui.heading("Vehicle and camera effects");
    ui.checkbox(&mut settings.tire_smoke, "Tire smoke");
    ui.add_enabled_ui(settings.tire_smoke, |ui| {
        choice(ui, "Smoke quality", &mut settings.smoke_quality, &[SmokeQuality::Standard, SmokeQuality::High]);
    });
    ui.checkbox(&mut settings.skid_marks, "Skid marks");
    ui.checkbox(&mut settings.collision_sparks, "Collision particles");
    ui.add_enabled_ui(settings.collision_sparks, |ui| {
        choice(
            ui,
            "Spark style",
            &mut settings.spark_style,
            &[SparkStyle::OriginalPc, SparkStyle::RestoredExperimental],
        );
    });
    ui.checkbox(&mut settings.speed_trails, "Speed trails (experimental)");
    ui.add_enabled_ui(settings.speed_trails, |ui| {
        choice(
            ui,
            "Speed trail style",
            &mut settings.speed_trail_style,
            &[SpeedTrailStyle::Streaks, SpeedTrailStyle::Car],
        );
    });
    ui.add_enabled_ui(settings.car_shading == crate::settings::CarShading::Glossy, |ui| {
        ui.checkbox(&mut settings.enhanced_lamps, "Enhanced vehicle lamps");
    });
    ui.checkbox(&mut settings.exhaust_flames, "Exhaust flames");
    ui.add_enabled_ui(settings.exhaust_flames && settings.renderer == crate::settings::RendererKind::Blackbox, |ui| {
        ui.checkbox(&mut settings.enhanced_exhaust, "Enhanced exhaust");
    });
    tail_trails(ui, &mut settings.tail_trail_style, preview);
    ui.checkbox(&mut settings.motion_blur, "Motion blur");
    ui.add_enabled_ui(settings.motion_blur, |ui| {
        ui.add(egui::Slider::new(&mut settings.motion_blur_strength, 0.0..=2.0).text("Motion blur strength"));
    });
    ui.small("Saved with F8 snapshots. Style, quality and Reset section retain these independent preferences.");
}

pub(super) fn content(ui: &mut egui::Ui, settings: &mut Settings) {
    ui.separator();
    ui.heading("HD content");
    ui.checkbox(&mut settings.custom_fonts.enabled, "Custom fonts");
    ui.checkbox(&mut settings.hd_content.fonts, "HD fonts");
    ui.checkbox(&mut settings.hd_content.cursor, "HD cursor");
    ui.checkbox(&mut settings.hd_content.fmv, "HD movies");
    ui.small("Uses your installed replacement content. Font asset changes apply on the next load; save and restart for consistent results.");
}

fn tail_trails(
    ui: &mut egui::Ui,
    style: &mut crate::settings::TailTrailStyle,
    preview: &mut Option<crate::settings::TailPreview>,
) {
    ui.separator();
    ui.checkbox(&mut style.enabled, "NOS taillight trails");
    ui.add_enabled_ui(style.enabled, |ui| {
        ui.checkbox(&mut style.enhanced, "Enhanced NOS taillight trails").on_hover_text(
            "Short curved ribbons following each rear lamp. Disable to compare the original PC flare copies.",
        );
        ui.add_enabled_ui(style.enhanced, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Appearance preset");
                for (label, preset) in [
                    ("Enhanced MW", crate::settings::TrailPreset::EnhancedMw),
                    ("Long Exposure", crate::settings::TrailPreset::LongExposure),
                    ("Pursuit", crate::settings::TrailPreset::Pursuit),
                ] {
                    if ui.button(label).clicked() { *style = preset.apply(*style); }
                }
            });
            ui.checkbox(&mut style.follow_lamp, "Follow lamp dimensions");
            let mut active = preview.is_some();
            if ui.checkbox(&mut active, "Preview NOS trail").changed() {
                *preview = active.then(crate::settings::TailPreview::default);
            }
            if let Some(value) = preview {
                ui.checkbox(&mut value.cycle, "Cycle boost engagement and release");
                ui.add(egui::Slider::new(&mut value.speed_kmh, 10..=400).text("Preview speed km/h"));
                ui.add(egui::Slider::new(&mut value.bend, -100..=100).text("Preview corner bend"));
                ui.small("World paused. Cosmetic preview only; close F8 to resume driving.");
            }
            choice(ui, "Trail shape", &mut style.shape, &[crate::settings::TrailShape::Ribbon, crate::settings::TrailShape::FlareRibbon]);
            for (label, value) in [
                ("Trail length %", &mut style.length),
                ("Trail exposure %", &mut style.exposure),
                ("Trail width %", &mut style.width),
                ("Trail brightness %", &mut style.brightness),
                ("Trail bloom %", &mut style.glow),
                ("Lamp bloom %", &mut style.lamp_bloom),
                ("Boost engagement %", &mut style.engagement),
                ("NOS lamp glow %", &mut style.lamp_glow),
                ("Lamp head size %", &mut style.head_size),
                ("Speed response %", &mut style.speed_response),
                ("Trail softness %", &mut style.softness),
                ("Trail shimmer %", &mut style.turbulence),
            ] {
                ui.add(egui::Slider::new(value, 0..=200).text(label));
            }
            ui.horizontal(|ui| {
                ui.label("Trail tint");
                ui.color_edit_button_srgb(&mut style.tint);
            });
            if ui.button("Reset trail appearance").clicked() {
                *style = crate::settings::TrailPreset::EnhancedMw.apply(*style);
            }
            ui.small("100%: 120 ms exposure, capped at 6 m. Exposure retains more of a corner; length limits distance. Shared across cars.");
        });
    });
}
