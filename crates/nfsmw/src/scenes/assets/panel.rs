//! The list of cars beside the asset viewer: a search box on top, the matching cars below, the chosen car
//! highlighted. Only the egui view lives here; the scene loads what is clicked.

use egui::{Align2, Color32, Frame, RichText, ScrollArea, TextEdit, vec2};

use super::PANEL_FRACTION;

/// Passes that re-centre the list on the selection: egui clamps the offset against the content size it knew last pass,
/// so the first passes settle it.
const SCROLL_PASSES: u8 = 6;

/// What the list keeps between passes: the search text, and whether the search box takes the keyboard.
pub struct Panel {
    pub query: String,
    /// Take the keyboard on the next pass. Set at the start, so the search can be typed into at once.
    focus: bool,
    /// Scroll so the selected name is in view on the next pass (the list opens on the selection).
    scroll: u8,
}

impl Default for Panel {
    fn default() -> Self {
        Self { query: String::new(), focus: true, scroll: SCROLL_PASSES }
    }
}

/// The width of the list in points: the strip on the left that the list takes.
pub fn width(ctx: &egui::Context) -> f32 {
    ctx.content_rect().width() * PANEL_FRACTION
}

/// Draw the list over the left of the window. `noun` names what is listed ("cars", "assets"). Returns the index
/// (into `names`) of the name clicked this pass.
pub fn show(
    ctx: &egui::Context,
    panel: &mut Panel,
    names: &[String],
    selected: usize,
    error: Option<&str>,
    noun: &str,
) -> Option<usize> {
    let screen = ctx.content_rect();
    let mut clicked = None;
    egui::Window::new("assets")
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .anchor(Align2::LEFT_TOP, vec2(0.0, 0.0))
        .fixed_size(vec2(width(ctx), screen.height()))
        .frame(Frame::new().fill(Color32::from_black_alpha(215)).inner_margin(8.0))
        .show(ctx, |ui| {
            let search = TextEdit::singleline(&mut panel.query)
                .hint_text(format!("search {noun}"))
                .desired_width(f32::INFINITY)
                .show(ui)
                .response;
            if std::mem::take(&mut panel.focus) {
                search.request_focus();
            }
            let shown = filter(names, &panel.query);
            ui.label(format!("{} of {} {noun}", shown.len(), names.len()));
            if let Some(error) = error {
                ui.label(RichText::new(error).color(Color32::LIGHT_RED));
            }
            ui.separator();
            // A row is its interact height; `show_rows` adds the gap between rows, so `row` is the whole pitch.
            let height = ui.spacing().interact_size.y;
            let row = height + ui.spacing().item_spacing.y;
            let mut area = ScrollArea::vertical().auto_shrink([false, false]);
            if panel.scroll > 0
                && let Some(at) = shown.iter().position(|&i| i == selected)
            {
                // Centre the selected row: rows are all one height, so its place in the list gives its offset.
                let offset = at as f32 * row - (ctx.content_rect().height() - row) * 0.5;
                area = area.vertical_scroll_offset(offset.max(0.0));
                panel.scroll -= 1;
            }
            area.show_rows(ui, height, shown.len(), |ui, range| {
                for &index in &shown[range] {
                    if ui.selectable_label(index == selected, &names[index]).clicked() {
                        clicked = Some(index);
                    }
                }
            });
        });
    clicked
}

/// The indices of the names that contain `query`, ignoring case. An empty query keeps them all.
pub fn filter(names: &[String], query: &str) -> Vec<usize> {
    let query = query.trim().to_ascii_lowercase();
    names.iter().enumerate().filter(|(_, name)| name.to_ascii_lowercase().contains(&query)).map(|(i, _)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cars() -> Vec<String> {
        ["BMWM3", "BMWM3GTR", "CAMARO", "CARRERAGT"].iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn an_empty_query_keeps_every_car() {
        assert_eq!(filter(&cars(), ""), vec![0, 1, 2, 3]);
        assert_eq!(filter(&cars(), "   "), vec![0, 1, 2, 3], "spaces around the query do not count");
    }

    #[test]
    fn the_search_ignores_case_and_matches_inside_the_name() {
        assert_eq!(filter(&cars(), "bmw"), vec![0, 1]);
        assert_eq!(filter(&cars(), "GTR"), vec![1]);
        assert_eq!(filter(&cars(), "rera"), vec![3], "inside the name, not only at its start");
        assert!(filter(&cars(), "zzz").is_empty());
    }
}
