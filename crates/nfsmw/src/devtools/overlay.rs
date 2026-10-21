//! The performance overlay: a small panel in the corner drawn from [`Metrics`].

use egui::{Align2, Color32, Frame, RichText, Sense, Shape, Stroke, pos2, vec2};

use super::metrics::{HISTORY, Metrics, ShowMetrics};

/// A frame time at which the graph tops out, in milliseconds (a 30 fps frame).
const GRAPH_MAX_MS: f32 = 33.3;

/// Draw the metrics `left` points in from the window's left edge (beside a scene's panel, or 0 for none).
pub fn show(ctx: &egui::Context, level: ShowMetrics, m: &Metrics, left: f32) {
    readout(ctx, m);
    if level == ShowMetrics::Off {
        return;
    }
    egui::Window::new("metrics")
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .interactable(false)
        .anchor(Align2::LEFT_TOP, vec2(left + 8.0, 8.0))
        .frame(Frame::window(&ctx.global_style()))
        .show(ctx, |ui| {
            let lines = rate_lines(m.fps(), m.average_ms(), m.presented_fps(), m.generation_configured);
            for (i, line) in lines.into_iter().enumerate() {
                let text = RichText::new(line).monospace();
                ui.label(if i == 0 { text.strong() } else { text });
            }
            ui.label(RichText::new(m.limit).color(Color32::LIGHT_BLUE));
            if level == ShowMetrics::Advanced {
                ui.label(
                    RichText::new(format!("Base 1% low {:.0} FPS  worst {:.1} ms", m.low_1pct_fps(), m.worst_ms()))
                        .monospace(),
                );
                let gpu = m.work.gpu_ms.map(|v| format!("{v:.2} ms")).unwrap_or_else(|| "unavailable".into());
                ui.label(
                    RichText::new(format!("CPU elapsed {:.2} ms · GPU base render {gpu}", m.work.cpu_ms))
                        .monospace(),
                );
                ui.label(
                    RichText::new(format!(
                        "Acquire {:.2} · Present {:.2} · Driver pacing {:.2} ms",
                        m.work.acquire_ms, m.work.present_ms, m.work.sleep_ms
                    ))
                    .monospace(),
                );
                let percent = |v: Option<f32>| v.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "measuring/unavailable".into());
                let activity = m.work.activity;
                ui.label(RichText::new(format!("Main thread active {} · GPU engine {} ({})",
                    percent(activity.main_thread_busy_percent), percent(activity.gpu_busy_percent), activity.gpu_engine)).monospace());
                ui.small(m.limit_hint);
                ui.small("OS activity: 1-second samples; GPU is this process's busiest engine, including FG/auxiliary queues.");
                ui.small("CPU elapsed can include waits. GPU base timestamp excludes FG/presentation and auxiliary views.");
                ui.small("Presented FPS uses SDK counts with FG; it is not the monitor's scanout rate.");
                let [ow, oh] = m.output_size;
                let [rw, rh] = m.render_size;
                ui.label(RichText::new(format!("Internal {rw}×{rh} → Game output {ow}×{oh}")).monospace());
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

/// The rate lines of the panel: the base (rendered) rate and frame time alone while nothing is generated, both rates and
/// frame times when the presented rate differs from the base (frame generation), and a line for frame generation only
/// when it is configured. The presented rate is `None` while it is being measured: nothing is said about it then.
fn rate_lines(base_fps: f32, base_ms: f32, presented_fps: Option<f32>, generation_configured: bool) -> Vec<String> {
    let presented = presented_fps.filter(|p| *p > 0.0 && p.round() != base_fps.round());
    let mut lines = match presented {
        None => vec![format!("Base: {base_fps:.0} FPS  {base_ms:.1} ms")],
        Some(p) => vec![
            format!("FPS: {base_fps:.0} base / {p:.0} presented"),
            format!("Frame time: {base_ms:.1} base / {:.1} presented ms", 1000.0 / p),
        ],
    };
    if generation_configured {
        lines.push("Frame generation: configured".to_owned());
    }
    lines
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
        .frame(Frame::window(&ctx.global_style()))
        .show(ctx, |ui| {
            for line in m.hud.lines() {
                ui.add(
                    egui::Label::new(RichText::new(line).monospace().size(18.0).strong())
                        .wrap_mode(egui::TextWrapMode::Extend),
                );
            }
        });
}

#[cfg(test)]
mod tests {
    use super::rate_lines;

    #[test]
    fn without_frame_generation_only_the_base_rate_and_time_show() {
        assert_eq!(rate_lines(144.0, 6.94, None, false), ["Base: 144 FPS  6.9 ms"]);
        // A presented rate equal to the base (after rounding) adds nothing.
        assert_eq!(rate_lines(144.0, 6.94, Some(144.2), false), ["Base: 144 FPS  6.9 ms"]);
    }

    #[test]
    fn a_different_presented_rate_shows_both_rates_and_times() {
        assert_eq!(
            rate_lines(100.0, 10.0, Some(200.0), true),
            [
                "FPS: 100 base / 200 presented",
                "Frame time: 10.0 base / 5.0 presented ms",
                "Frame generation: configured"
            ]
        );
    }

    #[test]
    fn frame_generation_shows_only_when_configured() {
        assert_eq!(rate_lines(60.0, 16.7, None, true).last().unwrap(), "Frame generation: configured");
        assert!(!rate_lines(60.0, 16.7, Some(120.0), false).iter().any(|l| l.contains("Frame generation")));
    }
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
