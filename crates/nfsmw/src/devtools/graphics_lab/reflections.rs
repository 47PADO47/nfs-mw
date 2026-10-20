//! Capture budgets and presentation, shared with the Video menu's value types.
use super::{GraphicsLab, view::choice};
use crate::settings::{MirrorTint, ReflectionSize, Settings, reflection_res};
use blackbox_gfx::reflection::{content::ReflectionContent, distance::ReflectionDistance, lod::ReflectionLod};

pub(super) fn show(ui: &mut egui::Ui, lab: &GraphicsLab, settings: &mut Settings) {
    ui.add_enabled_ui(lab.caps.is_some_and(|c| c.local_reflections), |ui| {
        ui.collapsing("Reflection capture settings", |ui| {
            cube_size(ui, settings, lab.output_size[1]);
            choice(ui, "Reflection detail", &mut settings.reflection_extra.lod, &ReflectionLod::ALL);
            choice(ui, "Reflection content", &mut settings.reflection_extra.content, &ReflectionContent::ALL);
            choice(ui, "Reflection distance", &mut settings.reflection_extra.distance, &ReflectionDistance::ALL);
            ui.add(
                egui::Slider::new(&mut settings.reflection_extra.brightness.0, 0.0..=3.0)
                    .text("Vehicle reflection brightness"),
            );
            size(ui, "Rear-view capture size", &mut settings.reflection_mirror_size);
            egui::ComboBox::from_label("Mirror tint")
                .selected_text(reflection_res::tint_label(settings.mirror_tint))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut settings.mirror_tint, MirrorTint::NONE, "Off");
                    ui.selectable_value(&mut settings.mirror_tint, MirrorTint::STOCK, "Stock");
                });
        });
    });
}

fn size(ui: &mut egui::Ui, label: &str, value: &mut ReflectionSize) {
    egui::ComboBox::from_label(label).selected_text(reflection_res::size_label(*value)).show_ui(ui, |ui| {
        for size in reflection_res::MENU_SIZES.into_iter().chain([128, 256, 512, 1024, 2048].map(ReflectionSize::Fixed))
        {
            ui.selectable_value(value, size, reflection_res::size_label(size));
        }
    });
}

fn cube_size(ui: &mut egui::Ui, settings: &mut Settings, output_height: u32) {
    let edge = reflection_res::cube_pixels(settings, output_height);
    egui::ComboBox::from_label("Car cubemap resolution").selected_text(format!("{edge} × {edge} per face")).show_ui(
        ui,
        |ui| {
            for edge in reflection_res::CUBE_SIZES {
                ui.selectable_value(
                    &mut settings.reflection_cube_size,
                    ReflectionSize::Fixed(edge),
                    format!("{edge} × {edge} per face"),
                );
            }
        },
    );
    if !matches!(settings.reflection_cube_size, ReflectionSize::Fixed(_)) {
        ui.small(reflection_res::cube_label(settings, output_height));
    }
    ui.small(format!("Six square faces at {edge} × {edge} pixels each. Resolution controls sharpness; High / Medium / Low also controls update rate."));
    ui.small("Fixed face sizes stay the same when game resolution or DLSS quality changes. Old percentage/Auto settings remain effective until you select a size.");
    if settings.reflections == blackbox_gfx::ReflectionQuality::Off {
        ui.small("Car reflections are off; no cubemap is captured.");
    }
}
