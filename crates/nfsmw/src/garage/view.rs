//! Runtime garage: a responsive vehicle browser beside live driving options.

use super::{Category, Model, Selection};
use crate::settings::Settings;

pub fn show(ctx: &egui::Context, open: &mut bool, model: &mut Model, settings: &mut Settings) -> Option<Selection> {
    model.draft.exhaust.set_style(settings.exhaust_style);
    let before = model.draft.clone();
    let (id, size, minimum) = match model.preview {
        true => ("garage_preview", egui::vec2(340.0, 680.0), egui::vec2(300.0, 400.0)),
        false => ("garage_window", egui::vec2(1000.0, 740.0), egui::vec2(620.0, 400.0)),
    };
    egui::Window::new("Garage")
        .id(egui::Id::new(id))
        .open(open)
        .default_size(size)
        .min_size(minimum)
        .default_pos(egui::pos2(24.0, 48.0))
        .resizable(true)
        .max_height((ctx.content_rect().height() - 70.0).max(300.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let label = match model.preview {
                    true => "Browse cars",
                    false => "Preview car",
                };
                if ui.button(label).clicked() {
                    model.preview = !model.preview;
                }
                if !model.preview {
                    ui.label("Changes apply immediately.");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak("F6 / Esc to close");
                });
            });
            super::presets::show(ui, model);
            model.draft.exhaust.set_style(settings.exhaust_style);
            ui.separator();
            let height = (ui.available_height() - 36.0).max(260.0);
            if model.preview {
                egui::ScrollArea::vertical().id_salt("garage_preview_options").max_height(height).show(ui, |ui| {
                    super::controls::show(ui, model, settings);
                });
                if let Some(Err(error)) = &model.message {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
                return;
            }
            let options_width = 300.0;
            let browser_width = (ui.available_width() - options_width - 16.0).max(300.0);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(browser_width, height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        browser(ui, model, height);
                    },
                );
                ui.add_space(8.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(options_width, height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical().id_salt("garage_options").max_height(height).show(ui, |ui| {
                            super::controls::show(ui, model, settings);
                        });
                    },
                );
            });
            ui.separator();
            if let Some(Err(error)) = &model.message {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            if !matches!(model.message, Some(Err(_))) {
                ui.weak("F5 controls camera framing during car swaps. Same-car changes preserve your position.");
            }
        });
    if model.exhaust_style_dirty && (!*open || !ctx.input(|i| i.pointer.any_down())) {
        model.exhaust_style_dirty = false;
        if let Some(path) = model.exhaust_style_path.as_ref() {
            let changes =
                crate::settings::Partial { exhaust_style: Some(settings.exhaust_style), ..Default::default() };
            if let Err(error) = crate::settings::write_file(path, &changes) {
                log::warn!("saving shared exhaust style: {error}");
                model.message = Some(Err(format!("Could not save exhaust style: {error}")));
            }
        }
    }
    if before == model.draft || model.selected().is_none_or(|car| car.unavailable.is_some()) {
        return None;
    }
    Some(model.draft.clone())
}

fn browser(ui: &mut egui::Ui, model: &mut Model, height: f32) {
    ui.horizontal(|ui| {
        for category in [Category::Racing, Category::Police] {
            let count = model.cars.iter().filter(|car| car.category == category).count();
            ui.selectable_value(&mut model.category, category, format!("{} ({count})", category.label()));
        }
    });
    ui.add_sized(
        egui::vec2(ui.available_width(), 28.0),
        egui::TextEdit::singleline(&mut model.filter).hint_text("Search by car or manufacturer"),
    );
    ui.add_space(6.0);
    let filter = model.filter.trim().to_ascii_lowercase();
    let matches: Vec<_> = model
        .cars
        .iter()
        .filter(|car| {
            car.category == model.category
                && car.unavailable.is_none()
                && format!("{} {} {}", car.name, car.manufacturer, car.folder).to_ascii_lowercase().contains(&filter)
        })
        .collect();
    ui.weak(format!("{} cars · click to drive", matches.len()));
    if matches.is_empty() {
        ui.label("No matching cars.");
        if ui.button("Clear search").clicked() {
            model.filter.clear();
        }
        return;
    }
    let mut picked = None;
    egui::ScrollArea::vertical()
        .id_salt(("garage_cars", model.category as u8))
        .max_height((height - 94.0).max(100.0))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let gap = 8.0;
            let columns = ((ui.available_width() + gap) / 168.0).floor().max(1.0) as usize;
            let width = (ui.available_width() - gap * (columns - 1) as f32) / columns as f32;
            egui::Grid::new(("garage_grid", model.category as u8))
                .num_columns(columns)
                .spacing(egui::vec2(gap, gap))
                .show(ui, |ui| {
                    for (index, car) in matches.iter().enumerate() {
                        let mut job = egui::text::LayoutJob::default();
                        job.append(
                            &car.name,
                            0.0,
                            egui::TextFormat {
                                font_id: egui::FontId::proportional(17.0),
                                color: ui.visuals().text_color(),
                                ..Default::default()
                            },
                        );
                        job.append(
                            &format!("\n{}", car.manufacturer),
                            0.0,
                            egui::TextFormat {
                                font_id: egui::FontId::proportional(14.0),
                                color: ui.visuals().weak_text_color(),
                                ..Default::default()
                            },
                        );
                        job.halign = egui::Align::LEFT;
                        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 62.0), egui::Sense::click());
                        let visuals = ui.style().interact_selectable(&response, car.folder == model.draft.car);
                        ui.painter().rect(
                            rect,
                            visuals.corner_radius,
                            visuals.bg_fill,
                            visuals.bg_stroke,
                            egui::StrokeKind::Inside,
                        );
                        job.wrap.max_width = (width - 20.0).max(1.0);
                        let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
                        let pos = egui::pos2(rect.left() + 10.0, rect.center().y - galley.size().y * 0.5);
                        ui.painter().galley(pos, galley, visuals.text_color());
                        if response.clicked() {
                            picked = Some(car.folder.clone());
                        }
                        response.on_hover_text(format!("{} · {}", car.category.label(), car.folder));
                        if (index + 1) % columns == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
    if let Some(car) = picked
        && car != model.draft.car
    {
        model.select(car);
    }
}
