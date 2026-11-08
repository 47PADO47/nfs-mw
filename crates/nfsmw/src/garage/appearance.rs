//! The appearance controls the vehicle panel hosts: vinyl colours, rim colour, gauges, plate.
use nfsmw_data::car::customization::{Catalog, Category};

use super::Model;
use crate::settings::{Settings, UiStyle};

/// What the vehicle panel's Appearance page hosts beside the options' own selectors (paint, tint, rim paint, vinyl): the vinyl
/// colour channels, the rim colour, the gauges, the licence plate, and a button that puts the whole appearance back to stock.
pub(crate) fn extras(ui: &mut egui::Ui, model: &mut Model, settings: &mut Settings) {
    let Some(catalog) = model.selected().and_then(|car| car.parts.clone()) else { return };
    ui.collapsing("Vinyl colours", |ui| {
        let count = model.draft.parts.parts[Category::Vinyl as usize]
            .and_then(|h| catalog.vinyl_remaps.get(&h))
            .copied()
            .unwrap_or(0);
        for (i, vinyl) in model.draft.parts.vinyl_colors.iter_mut().enumerate().take(count.min(3)) {
            let previous = *vinyl;
            color(ui, &format!("Vinyl color {}", i + 1), vinyl);
            if *vinyl != previous {
                model.draft.parts.vinyl_color_parts[i] = None;
            }
        }
        if count == 0 {
            ui.weak("Pick a recolorable vinyl to see its colour channels.");
        }
    });
    ui.collapsing("Rim colour", |ui| rims(ui, model, &catalog));
    ui.collapsing("Gauges", |ui| gauges(ui, model, &catalog, settings));
    super::plate_edit::show(ui, model, &catalog);
    if ui.button("Restore stock appearance").clicked() {
        for category in [Category::Paint, Category::WindowTint, Category::RimPaint, Category::Vinyl] {
            model.draft.parts.parts[category as usize] = None;
        }
        model.draft.parts.rim_color = None;
        model.draft.parts.vinyl_colors = [None; 3];
        model.draft.parts.vinyl_color_parts = [None; 3];
        model.draft.gauges = Default::default();
        model.draft.color = None;
    }
}

fn rims(ui: &mut egui::Ui, model: &mut Model, catalog: &Catalog) {
    let eligible =
        model.draft.parts.parts[Category::Wheels as usize].is_some_and(|h| catalog.paintable_wheels.contains(&h));
    ui.add_enabled_ui(eligible || catalog.custom_rim_paint, |ui| {
        color(ui, "Rim color", &mut model.draft.parts.rim_color);
    });
    if !eligible && !catalog.custom_rim_paint {
        ui.weak("Fit aftermarket rims on the Parts page to paint them.");
    }
}

fn gauges(ui: &mut egui::Ui, model: &mut Model, catalog: &Catalog, settings: &mut Settings) {
    egui::ComboBox::from_id_salt("garage_gauge_skin")
        .selected_text(format!("Gauge style {}", model.draft.gauges.skin))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut model.draft.gauges.skin, 0, "Stock");
            for &skin in catalog.gauge_skins.iter().filter(|s| **s != 0) {
                ui.selectable_value(&mut model.draft.gauges.skin, skin, format!("Style {skin}"));
            }
        });
    let before = model.draft.gauges.colors();
    color(ui, "Backing", &mut model.draft.gauges.backing);
    color(ui, "Needle", &mut model.draft.gauges.needle);
    color(ui, "Digits", &mut model.draft.gauges.digits);
    for (index, color) in model.draft.gauges.colors().into_iter().enumerate() {
        if color != before[index] {
            model.draft.gauges.color_parts[index] = None;
        }
    }
    if ui.button("Restore stock gauges").clicked() {
        model.draft.gauges = Default::default();
    }
    if settings.ui_style != UiStyle::Original {
        ui.weak("Gauge styles and colors use the Original HUD theme.");
        if ui.button("Use Original HUD").clicked() {
            settings.ui_style = UiStyle::Original;
        }
    }
}

fn color(ui: &mut egui::Ui, label: &str, value: &mut Option<[u8; 3]>) {
    ui.push_id(label, |ui| {
        ui.horizontal(|ui| {
            ui.label(label);
            if value.is_none() {
                ui.weak("Preset");
            }
            let mut rgb = value.unwrap_or([255; 3]);
            if ui.color_edit_button_srgb(&mut rgb).changed() {
                *value = Some(rgb);
            }
            if ui.add_enabled(value.is_some(), egui::Button::new("Reset")).clicked() {
                *value = None;
            }
        })
    });
}
