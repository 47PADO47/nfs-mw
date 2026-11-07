//! The stepper: one value of a list on one line, changed by clicking its sides, the arrow keys or the controller's D-pad.

use egui::{Color32, EventFilter, Key, Sense, Stroke, StrokeKind, TextStyle, Ui, vec2};

/// How a stepper was changed.
pub(super) fn next_index(index: usize, count: usize, forward: bool) -> usize {
    if count == 0 {
        return 0;
    }
    match forward {
        true => (index + 1) % count,
        false => (index + count - 1) % count,
    }
}

/// A line `◀ value ▶` for the item `index` of `count`. Returns the index asked for, if the stepper was changed. The left and right
/// arrow keys change it while it has the keyboard focus, Enter takes the next one; a click on its left or right half takes the
/// previous or the next. `want_focus` gives it the keyboard focus (the first line of a panel a controller opens).
pub(super) fn stepper(
    ui: &mut Ui,
    id: egui::Id,
    value: &str,
    index: usize,
    count: usize,
    want_focus: bool,
) -> Option<usize> {
    let height = ui.text_style_height(&TextStyle::Body) + 10.0;
    let size = vec2(ui.available_width().max(160.0), height);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let response = ui.interact(rect, id, Sense::click());
    if want_focus {
        response.request_focus();
    }
    let focused = response.has_focus();
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
    if focused {
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(id, EventFilter { horizontal_arrows: true, ..Default::default() });
        });
    }
    let mut asked = None;
    if response.clicked() {
        let left = response.interact_pointer_pos().is_some_and(|at| at.x < rect.center().x);
        asked = Some(next_index(index, count, !left));
    }
    if focused {
        let (left, right, enter) =
            ui.input(|i| (i.key_pressed(Key::ArrowLeft), i.key_pressed(Key::ArrowRight), i.key_pressed(Key::Enter)));
        if left {
            asked = Some(next_index(index, count, false));
        }
        if right || enter {
            asked = Some(next_index(index, count, true));
        }
    }
    paint(ui, &response, value, index, count, focused);
    asked.filter(|_| count > 1)
}

fn paint(ui: &Ui, response: &egui::Response, value: &str, index: usize, count: usize, focused: bool) {
    let visuals = ui.style().interact(response);
    let rect = response.rect;
    let stroke = match focused {
        true => Stroke::new(2.0, ui.visuals().selection.stroke.color),
        false => visuals.bg_stroke,
    };
    ui.painter().rect(rect, visuals.corner_radius, visuals.bg_fill, stroke, StrokeKind::Inside);
    let font = TextStyle::Body.resolve(ui.style());
    let ink = visuals.text_color();
    let arrow = |text: &str, at: egui::Pos2, align: egui::Align2| {
        ui.painter().text(
            at,
            align,
            text,
            font.clone(),
            match count > 1 {
                true => ink,
                false => Color32::from_gray(100),
            },
        );
    };
    arrow("◀", rect.left_center() + vec2(8.0, 0.0), egui::Align2::LEFT_CENTER);
    arrow("▶", rect.right_center() - vec2(8.0, 0.0), egui::Align2::RIGHT_CENTER);
    let shown = format!("{value}   {}/{count}", index + 1);
    let width = rect.width() - 48.0;
    let galley = ui.painter().layout(shown, font, ink, width);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, ink);
}

#[cfg(test)]
mod tests {
    use super::next_index;

    #[test]
    fn stepping_wraps_round_the_list() {
        assert_eq!(next_index(0, 3, true), 1);
        assert_eq!(next_index(2, 3, true), 0);
        assert_eq!(next_index(0, 3, false), 2);
        assert_eq!(next_index(1, 3, false), 0);
        assert_eq!(next_index(0, 0, true), 0);
        assert_eq!(next_index(0, 1, true), 0);
    }
}
