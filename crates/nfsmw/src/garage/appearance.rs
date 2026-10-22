//! Installed fixed artwork and quick finish/color controls.
use nfsmw_data::car::customization::{Catalog, Category};

use super::Model;
use crate::settings::{Settings, UiStyle};

pub(super) fn body_color(model: &Model) -> Option<[u8; 3]> {
    let car = model.selected()?;
    model
        .draft
        .color
        .or_else(|| car.parts.as_ref()?.colors.get(&model.draft.parts.parts[Category::Paint as usize]?).copied())
        .or(car.original_color)
}

fn finish(model: &mut Model, hash: u32) {
    model.draft.color = body_color(model);
    model.draft.parts.parts[Category::Paint as usize] = Some(hash);
}

pub(super) fn show(ui: &mut egui::Ui, model: &mut Model, settings: &mut Settings) {
    let Some(catalog) = model.selected().and_then(|car| car.parts.clone()) else { return };
    ui.heading("Paint finish");
    ui.horizontal_wrapped(|ui| {
        for (label, hash) in &catalog.finishes {
            let active = model.draft.parts.parts[Category::Paint as usize] == Some(*hash);
            if ui
                .selectable_label(active, label)
                .on_hover_text("Keep your current color and change the finish")
                .clicked()
            {
                finish(model, *hash);
            }
        }
    });
    for category in [Category::Paint, Category::WindowTint] {
        super::parts::selector(ui, model, &catalog, category);
    }
    ui.collapsing("Vinyls", |ui| {
        super::parts::selector(ui, model, &catalog, Category::Vinyl);
        let count = model.draft.parts.parts[Category::Vinyl as usize]
            .and_then(|h| catalog.vinyl_remaps.get(&h))
            .copied()
            .unwrap_or(0);
        for (i, vinyl) in model.draft.parts.vinyl_colors.iter_mut().enumerate().take(count.min(3)) {
            color(ui, &format!("Vinyl color {}", i + 1), vinyl);
        }
        ui.weak("Fixed artwork. Color controls appear for recolorable vinyls.");
    });
    ui.collapsing("Rim paint", |ui| rims(ui, model, &catalog));
    ui.collapsing("Gauges", |ui| gauges(ui, model, &catalog, settings));
    if ui.button("Restore stock appearance").clicked() {
        for category in [Category::Paint, Category::WindowTint, Category::RimPaint, Category::Vinyl] {
            model.draft.parts.parts[category as usize] = None;
        }
        model.draft.parts.rim_color = None;
        model.draft.parts.vinyl_colors = [None; 3];
        model.draft.gauges = Default::default();
        model.draft.color = None;
    }
}

fn rims(ui: &mut egui::Ui, model: &mut Model, catalog: &Catalog) {
    let eligible =
        model.draft.parts.parts[Category::Wheels as usize].is_some_and(|h| catalog.paintable_wheels.contains(&h));
    ui.add_enabled_ui(eligible, |ui| {
        super::parts::selector(ui, model, catalog, Category::RimPaint);
        color(ui, "Rim color", &mut model.draft.parts.rim_color);
    });
    if !eligible {
        ui.weak("Fit aftermarket rims in Parts to paint them.");
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
    color(ui, "Backing", &mut model.draft.gauges.backing);
    color(ui, "Needle", &mut model.draft.gauges.needle);
    color(ui, "Digits", &mut model.draft.gauges.digits);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_finish_preserves_custom_or_current_preset_color() {
        let mut catalog = Catalog::default();
        catalog.colors.insert(42, [12, 34, 56]);
        let mut model = Model::default();
        model.cars.push(super::super::Car {
            folder: "CAR".into(),
            original_color: Some([80; 3]),
            parts: Some(catalog),
            ..Default::default()
        });
        model.select("CAR".into());
        model.draft.parts.parts[Category::Paint as usize] = Some(42);
        finish(&mut model, 43);
        assert_eq!(model.draft.color, Some([12, 34, 56]));
        model.draft.color = Some([200, 10, 50]);
        finish(&mut model, 44);
        assert_eq!(model.draft.color, Some([200, 10, 50]));
    }
}
