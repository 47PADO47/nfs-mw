//! The mini player along the bottom centre of the window: previous, play or pause, next, the sound's name, and a
//! slider that follows the playhead. Dragging the slider seeks when the drag ends.

use egui::{Align2, Button, Color32, Frame, RichText, Slider, vec2};

use crate::audio::preview::Status;

/// What the player asks for this pass.
#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Previous,
    Next,
    TogglePause,
    /// Move the playhead to this many seconds.
    Seek(f64),
}

/// Formats seconds as `m:ss.cc`.
pub fn time(seconds: f64) -> String {
    let total = seconds.max(0.0);
    let minutes = (total / 60.0).floor();
    format!("{:01}:{:05.2}", minutes as u64, total - minutes * 60.0)
}

/// Draws the mini player centred along the bottom of the window, to the right of the list (`left` is the width the
/// list takes). `seek` is the slider's value and `seeking` says whether the user is dragging it.
pub fn show(
    ctx: &egui::Context,
    name: Option<&str>,
    status: &Status,
    seek: &mut f64,
    seeking: &mut bool,
    left: f64,
) -> Action {
    let screen = ctx.content_rect();
    let width = (screen.width() - left as f32 - 32.0).clamp(240.0, 640.0);
    let mut action = Action::None;
    egui::Window::new("player")
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .anchor(Align2::CENTER_BOTTOM, vec2(left as f32 / 2.0, -12.0))
        .fixed_size(vec2(width, 0.0))
        .frame(Frame::new().fill(Color32::from_black_alpha(225)).inner_margin(10.0).corner_radius(6.0))
        .show(ctx, |ui| {
            let label = name.unwrap_or("pick a sound");
            ui.label(RichText::new(label).strong());
            if let Some(error) = &status.error {
                ui.label(RichText::new(error).color(Color32::LIGHT_RED));
            }
            ui.horizontal(|ui| {
                if ui.add(Button::new("⏮").min_size(vec2(36.0, 28.0))).clicked() {
                    action = Action::Previous;
                }
                let pause_label = if status.playing { "⏸" } else { "▶" };
                if ui.add(Button::new(pause_label).min_size(vec2(36.0, 28.0))).clicked() {
                    action = Action::TogglePause;
                }
                if ui.add(Button::new("⏭").min_size(vec2(36.0, 28.0))).clicked() {
                    action = Action::Next;
                }
                let shown_at = if *seeking { *seek } else { status.position };
                ui.label(format!("{} / {}", time(shown_at), time(status.duration)));
            });
            let max = status.duration.max(0.001);
            ui.spacing_mut().slider_width = width - 24.0;
            let response = ui
                .add_enabled(status.duration > 0.0, Slider::new(seek, 0.0..=max).show_value(false).trailing_fill(true));
            if response.dragged() {
                *seeking = true;
            }
            if response.drag_stopped() || (response.changed() && !response.dragged()) {
                *seeking = false;
                action = Action::Seek(*seek);
            }
        });
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_is_minutes_seconds_and_hundredths() {
        assert_eq!(time(0.0), "0:00.00");
        assert_eq!(time(4.5), "0:04.50");
        assert_eq!(time(65.25), "1:05.25");
        assert_eq!(time(-1.0), "0:00.00", "a negative playhead reads as the start");
    }
}
