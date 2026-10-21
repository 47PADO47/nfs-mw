//! Particle and camera effects are independent of visual style and quality presets.
use super::view::choice;
use crate::settings::{Settings, SmokeQuality, SparkStyle, SpeedTrailStyle};

pub(super) fn show(ui: &mut egui::Ui, settings: &mut Settings) {
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
    ui.checkbox(&mut settings.exhaust_flames, "Exhaust flames");
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
