//! Quick session presets and reversible comparison snapshots.
use super::quality::Quality;
use super::*;

impl GraphicsLab {
    pub(super) fn remember_startup(&mut self, settings: &Settings) {
        self.original.get_or_insert((settings.appearance.lighting, *settings));
    }
    pub(super) fn restore_startup(&mut self, settings: &mut Settings) {
        let Some((options, snapshot)) = self.original else { return };
        self.options = options;
        restore_graphics(settings, &snapshot);
    }
    fn select_quality(&mut self, quality: Quality, settings: &mut Settings) {
        self.options = RenderExperiments { freeze_sun: self.options.freeze_sun, ..quality.apply(settings) };
    }
    fn experiment_preset(&mut self, preset: &str) {
        self.options = RenderExperiments {
            freeze_sun: self.options.freeze_sun,
            lighting_debug: self.options.lighting_debug,
            ..preset_options(preset)
        };
    }
}

pub(super) fn hotkeys(
    mut lab: ResMut<GraphicsLab>,
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<Settings>,
    gui: Option<Res<crate::gui::Gui>>,
) {
    if !lab.open || gui.is_some_and(|gui| gui.ctx.egui_wants_keyboard_input()) {
        return;
    }
    lab.remember_startup(&settings);
    for (key, preset) in
        [(KeyCode::Digit1, Quality::Low), (KeyCode::Digit2, Quality::Balanced), (KeyCode::Digit3, Quality::High)]
    {
        if keys.just_pressed(key) {
            lab.select_quality(preset, &mut settings);
        }
    }
    if keys.just_pressed(KeyCode::Digit0) {
        lab.restore_startup(&mut settings);
    }
}

pub(super) fn show(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &mut Settings) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Quick quality:");
        for (label, preset) in
            [("Low · 1", Quality::Low), ("Balanced · 2", Quality::Balanced), ("High · 3", Quality::High)]
        {
            if ui.button(label).clicked() {
                lab.select_quality(preset, settings);
            }
        }
        if ui.button("Restore startup · 0").clicked() {
            lab.restore_startup(settings);
        }
    });
    if lab.tab == profiles::Tab::Lighting {
        ui.horizontal_wrapped(|ui| {
            ui.label("Lighting presets:");
            for (name, preset) in [
                ("Off", "baseline"),
                ("Sky", "ambient"),
                ("Sun", "shadows"),
                ("Far", "distance"),
                ("Detail", "detail"),
                ("Coat", "modern"),
            ] {
                if ui.button(name).clicked() {
                    lab.experiment_preset(preset);
                }
            }
        });
    }
    ui.horizontal_wrapped(|ui| {
        if ui.button("Store A").clicked() {
            lab.saved = Some((lab.options, *settings));
        }
        if ui.add_enabled(lab.saved.is_some(), egui::Button::new("Swap A/B")).clicked() {
            let (options, snapshot) = lab.saved.replace((lab.options, *settings)).expect("enabled saved state");
            lab.options = options;
            restore_graphics(settings, &snapshot);
        }
    });
}

pub(super) fn restore_graphics(settings: &mut Settings, snapshot: &Settings) {
    restore_appearance(settings, snapshot);
    settings.render_scale = snapshot.render_scale;
    settings.upscaler = snapshot.upscaler;
    settings.ray_tracing = snapshot.ray_tracing;
    settings.tire_smoke = snapshot.tire_smoke;
    settings.skid_marks = snapshot.skid_marks;
    settings.smoke_quality = snapshot.smoke_quality;
    settings.collision_sparks = snapshot.collision_sparks;
    settings.speed_trails = snapshot.speed_trails;
}

pub(super) fn restore_appearance(settings: &mut Settings, snapshot: &Settings) {
    settings.appearance = snapshot.appearance;
    settings.graphics_preset = snapshot.graphics_preset;
    settings.car_shading = snapshot.car_shading;
    settings.shadows = snapshot.shadows;
    settings.reflections = snapshot.reflections;
    settings.ao = snapshot.ao;
    settings.ao_intensity = snapshot.ao_intensity;
    settings.ao_compare = snapshot.ao_compare;
    settings.post_tonemap = snapshot.post_tonemap;
    settings.post_bloom = snapshot.post_bloom;
    settings.post_aa = snapshot.post_aa;
    settings.visual_treatment = snapshot.visual_treatment;
}
