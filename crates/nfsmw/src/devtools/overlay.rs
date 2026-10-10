//! The performance overlay: a small panel in the corner drawn from [`Metrics`].

use egui::{Align2, Color32, Frame, RichText, Sense, Shape, Stroke, pos2, vec2};

use super::metrics::{HISTORY, Metrics, ShowMetrics};

/// A frame time at which the graph tops out, in milliseconds (a 30 fps frame).
const GRAPH_MAX_MS: f32 = 33.3;

pub fn show(ctx: &egui::Context, level: ShowMetrics, m: &Metrics) {
    readout(ctx, m);
    if level == ShowMetrics::Off {
        return;
    }
    egui::Window::new("metrics")
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .interactable(false)
        .anchor(Align2::LEFT_TOP, vec2(8.0, 8.0))
        .frame(Frame::window(&ctx.global_style()).fill(Color32::from_black_alpha(170)))
        .show(ctx, |ui| {
            let headline = format!("{:.0} fps  {:.1} ms", m.fps(), m.average_ms());
            ui.label(RichText::new(headline).monospace().strong());
            if level == ShowMetrics::Advanced {
                ui.label(
                    RichText::new(format!("1% low {:.0} fps  worst {:.1} ms", m.low_1pct_fps(), m.worst_ms()))
                        .monospace(),
                );
                graph(ui, m);
                ui.label(RichText::new(format!("{} meshes  {} textures", m.meshes, m.textures)).monospace());
                if !m.adapter.is_empty() {
                    ui.label(RichText::new(&m.adapter).monospace().weak());
                }
                if !m.status.is_empty() {
                    ui.label(RichText::new(&m.status).monospace().weak());
                }
            }
        });
}

/// The scene's own readout (speed, rpm and gear when driving), bottom left.
fn readout(ctx: &egui::Context, m: &Metrics) {
    if m.hud.is_empty() {
        return;
    }
    egui::Window::new("readout")
        .fade_in(false)
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .interactable(false)
        .anchor(Align2::LEFT_BOTTOM, vec2(8.0, -8.0))
        .frame(Frame::window(&ctx.global_style()).fill(Color32::from_black_alpha(170)))
        .show(ctx, |ui| {
            for line in m.hud.lines() {
                ui.add(
                    egui::Label::new(RichText::new(line).monospace().size(18.0).strong())
                        .wrap_mode(egui::TextWrapMode::Extend),
                );
            }
        });
}

/// Frame times as a line, with a mark at 60 fps.
fn graph(ui: &mut egui::Ui, m: &Metrics) {
    let (rect, _) = ui.allocate_exact_size(vec2(HISTORY as f32, 48.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let y_of = |ms: f32| rect.bottom() - (ms / GRAPH_MAX_MS).min(1.0) * rect.height();
    let y60 = y_of(1000.0 / 60.0);
    painter.line_segment(
        [pos2(rect.left(), y60), pos2(rect.right(), y60)],
        Stroke::new(1.0, Color32::from_white_alpha(50)),
    );
    let offset = HISTORY - m.frame_ms.len();
    let points =
        m.frame_ms.iter().enumerate().map(|(i, &ms)| pos2(rect.left() + (offset + i) as f32, y_of(ms))).collect();
    painter.add(Shape::line(points, Stroke::new(1.0, Color32::LIGHT_GREEN)));
}
