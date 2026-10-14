//! Live camera and paint controls, kept separate from scrolling through cars.

use super::Model;
use crate::settings::{CameraMode, Settings};

pub(super) fn show(ui: &mut egui::Ui, model: &mut Model, settings: &mut Settings) {
    let selected = model.selected();
    let title = selected.map_or(model.draft.car.as_str(), |car| car.name.as_str());
    ui.heading(title);
    if let Some(car) = selected {
        let subtitle = match car.category {
            super::Category::Racing => car.manufacturer.clone(),
            super::Category::Police => "Police vehicle".to_owned(),
        };
        ui.weak(subtitle);
    }
    ui.add_space(12.0);
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(&mut model.options_tab, super::OptionsTab::Performance, "Performance");
        ui.selectable_value(&mut model.options_tab, super::OptionsTab::Tuning, "Tuning");
        ui.selectable_value(&mut model.options_tab, super::OptionsTab::Appearance, "Appearance");
        ui.selectable_value(&mut model.options_tab, super::OptionsTab::Parts, "Parts");
    });
    ui.separator();
    if model.options_tab == super::OptionsTab::Parts {
        super::parts::show(ui, model);
        return;
    }
    if model.options_tab == super::OptionsTab::Performance {
        super::performance::show(ui, model);
        return;
    }
    if model.options_tab == super::OptionsTab::Tuning {
        super::tuning::show(ui, model);
        return;
    }
    ui.heading("Camera view");
    egui::ComboBox::from_id_salt("garage_camera")
        .width(ui.available_width())
        .selected_text(camera_label(settings.camera_mode))
        .show_ui(ui, |ui| {
            for camera in CameraMode::ALL {
                ui.selectable_value(&mut settings.camera_mode, camera, camera_label(camera));
            }
        });
    ui.add_space(16.0);
    ui.heading("Body color");
    let paintable = model.selected().is_some_and(|car| car.paintable);
    let original = model.selected().and_then(|car| car.original_color).unwrap_or([190, 35, 35]);
    ui.add_enabled_ui(paintable, |ui| {
        ui.horizontal(|ui| {
            if ui.selectable_label(model.draft.color.is_none(), "Original paint").clicked() {
                model.draft.color = None;
            }
            if ui.selectable_label(model.draft.color.is_some(), "Custom paint").clicked() && model.draft.color.is_none()
            {
                model.draft.color = Some(original);
            }
        });
        ui.add_space(6.0);
        egui::Grid::new("garage_swatches").num_columns(3).spacing(egui::vec2(5.0, 5.0)).show(ui, |ui| {
            for (index, (label, rgb)) in [
                ("Red", [190, 35, 35]),
                ("Blue", [30, 85, 210]),
                ("Black", [20, 20, 24]),
                ("White", [235, 235, 235]),
                ("Silver", [140, 148, 158]),
                ("Yellow", [235, 190, 25]),
            ]
            .into_iter()
            .enumerate()
            {
                let ink = match index >= 3 {
                    true => egui::Color32::BLACK,
                    false => egui::Color32::WHITE,
                };
                if ui
                    .add_sized(
                        egui::vec2(68.0, 28.0),
                        egui::Button::new(egui::RichText::new(label).color(ink))
                            .fill(egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])),
                    )
                    .clicked()
                {
                    model.draft.color = Some(rgb);
                }
                if index % 3 == 2 {
                    ui.end_row();
                }
            }
        });
        if let Some(rgb) = &mut model.draft.color {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Custom color");
                ui.color_edit_button_srgb(rgb);
            });
        }
    });
    if !paintable {
        ui.weak("This car has a fixed livery.");
    }
}

fn camera_label(mode: CameraMode) -> &'static str {
    match mode {
        CameraMode::Chase => "Chase",
        CameraMode::DynamicFov => "Dynamic field of view",
        CameraMode::Bumper => "Bumper",
        CameraMode::Hood => "Hood",
        CameraMode::Close => "Close chase",
        CameraMode::Far => "Far chase",
        CameraMode::SuperFar => "Extra far chase",
        CameraMode::Drift => "Drift",
        CameraMode::Pursuit => "Pursuit",
    }
}
