//! The devtools menu: every developer panel on one list a controller can walk, and the line of buttons the panels show.

use egui::{Align2, RichText, vec2};

use super::metrics::ShowMetrics;
use super::panels::Panel;

/// What a line of the menu asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuAction {
    Open(Panel),
    /// The performance overlay to its next level: off, basic, advanced.
    CycleMetrics,
    /// A performance trace: start it, or stop the one that runs.
    ToggleTrace,
}

/// What the menu shows besides the panels.
pub(crate) struct MenuView {
    /// The scene has a garage to open.
    pub garage: bool,
    pub metrics: ShowMetrics,
    pub recording: bool,
    /// The name of a `--hud-scenario` run.
    pub scenario: Option<&'static str>,
    /// A controller is connected: the first line takes the focus.
    pub pad: bool,
}

/// The next level of the metrics overlay.
pub(crate) fn next_metrics(metrics: ShowMetrics) -> ShowMetrics {
    match metrics {
        ShowMetrics::Off => ShowMetrics::Basic,
        ShowMetrics::Basic => ShowMetrics::Advanced,
        ShowMetrics::Advanced => ShowMetrics::Off,
    }
}

/// The buttons of a controller and what they do in the panels, as one line.
pub(crate) const HINT: &str =
    "D-pad / left stick: move   A: select   B: back   LB / RB: previous / next   L3 + R3: devtools menu";

pub(crate) fn show(ctx: &egui::Context, view: &MenuView) -> Vec<MenuAction> {
    let mut actions = Vec::new();
    egui::Window::new("Devtools")
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.heading("Devtools");
            ui.weak("Every panel on one list. B or L3 + R3 closes it.");
            ui.add_space(6.0);
            let mut first = true;
            let mut line = |ui: &mut egui::Ui, text: String, enabled: bool, action: MenuAction| {
                let response = ui.add_enabled(enabled, egui::Button::new(text).min_size(vec2(320.0, 28.0)));
                let wants_focus =
                    std::mem::take(&mut first) && view.pad && ctx.memory(|memory| memory.focused().is_none());
                if wants_focus {
                    response.request_focus();
                }
                if response.clicked() {
                    actions.push(action);
                }
            };
            line(ui, format!("Vehicle options   ({})", Panel::Vehicle.key()), true, MenuAction::Open(Panel::Vehicle));
            for panel in [Panel::Console, Panel::GraphicsLab, Panel::ExtraOptions] {
                line(ui, format!("{}   ({})", panel.label(), panel.key()), true, MenuAction::Open(panel));
            }
            line(ui, format!("Garage   ({})", Panel::Garage.key()), view.garage, MenuAction::Open(Panel::Garage));
            ui.separator();
            line(ui, format!("Metrics overlay: {}", view.metrics), true, MenuAction::CycleMetrics);
            let trace = match view.recording {
                true => "Trace: stop recording (F9)",
                false => "Trace: start recording (F9)",
            };
            line(ui, trace.to_owned(), true, MenuAction::ToggleTrace);
            if let Some(name) = view.scenario {
                ui.separator();
                ui.weak(format!("HUD scenario running: {name} (--hud-scenario)"));
            }
            ui.add_space(6.0);
            ui.weak(HINT);
        });
    actions
}

/// The buttons of the controller, over the panels, while one is connected and a panel is up.
pub(crate) fn hint(ctx: &egui::Context) {
    egui::Area::new("devtools_pad_hint".into()).anchor(Align2::CENTER_BOTTOM, vec2(0.0, -10.0)).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.label(RichText::new(HINT).small());
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_metrics_overlay_cycles_through_its_levels() {
        let mut level = ShowMetrics::Off;
        let mut seen = Vec::new();
        for _ in 0..3 {
            level = next_metrics(level);
            seen.push(level);
        }
        assert_eq!(seen, [ShowMetrics::Basic, ShowMetrics::Advanced, ShowMetrics::Off]);
    }

    #[test]
    fn every_panel_has_a_word_and_a_key_and_the_word_finds_it_again() {
        for panel in Panel::ALL {
            assert_eq!(Panel::of_word(panel.word()), Some(panel));
            assert!(!panel.key().is_empty() && !panel.label().is_empty());
        }
        assert_eq!(Panel::of_word("nothing"), None);
    }
}
