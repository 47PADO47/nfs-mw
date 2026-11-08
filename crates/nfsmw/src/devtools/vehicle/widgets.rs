//! The stepper: one value of a list on one line, changed by clicking its sides, the arrow keys or the controller's D-pad, with a
//! list of every value beside it for a mouse.

use egui::{Color32, EventFilter, Key, Sense, Stroke, StrokeKind, TextStyle, Ui, vec2};

/// How many entries Page Up and Page Down move by.
const JUMP: usize = 10;
/// A list with more entries than this has a search box.
const SEARCH_ABOVE: usize = 12;

/// The entry after (or before) `index` of `count`, wrapping round the list.
pub(super) fn next_index(index: usize, count: usize, forward: bool) -> usize {
    if count == 0 {
        return 0;
    }
    match forward {
        true => (index + 1) % count,
        false => (index + count - 1) % count,
    }
}

/// The entry `JUMP` places on, stopping at the ends of the list.
pub(super) fn jump_index(index: usize, count: usize, forward: bool) -> usize {
    match forward {
        true => (index + JUMP).min(count.saturating_sub(1)),
        false => index.saturating_sub(JUMP),
    }
}

/// A stepper with the list of its values beside it. `label_of` names the value at a position, only when the list is open. The
/// values are numbered from `from` (the number the console takes: 0 for a slot's stock choice, 1 for the rider's own gear).
/// Returns the position asked for, if the value was changed.
pub(super) fn chooser(
    ui: &mut Ui,
    id: egui::Id,
    value: &str,
    (index, count, from): (usize, usize, usize),
    want_focus: bool,
    label_of: &dyn Fn(usize) -> String,
) -> Option<usize> {
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 34.0).max(120.0);
        let mut asked = ui
            .scope(|ui| {
                ui.set_width(width);
                stepper(ui, id, value, (index, count, from), want_focus)
            })
            .inner;
        if count > 2
            && let Some(at) = list(ui, id, (index, count, from), label_of)
        {
            asked = Some(at);
        }
        asked
    })
    .inner
}

/// A line `◀ value ▶` for the item `index` of `count`. Returns the index asked for, if the stepper was changed. The left and right
/// arrow keys change it while it has the keyboard focus, Page Up and Page Down ten at a time, Enter takes the next one; a click on
/// its left or right half takes the previous or the next. `want_focus` gives it the keyboard focus (the first line of a panel a
/// controller opens).
fn stepper(ui: &mut Ui, id: egui::Id, value: &str, shown: (usize, usize, usize), want_focus: bool) -> Option<usize> {
    let (index, count, _) = shown;
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
        let keys = |key: Key| ui.input(|i| i.key_pressed(key));
        if keys(Key::ArrowLeft) {
            asked = Some(next_index(index, count, false));
        }
        if keys(Key::ArrowRight) || keys(Key::Enter) {
            asked = Some(next_index(index, count, true));
        }
        if keys(Key::PageUp) {
            asked = Some(jump_index(index, count, false));
        }
        if keys(Key::PageDown) {
            asked = Some(jump_index(index, count, true));
        }
    }
    paint(ui, &response, value, shown, focused);
    asked.filter(|_| count > 1)
}

fn paint(ui: &Ui, response: &egui::Response, value: &str, (index, count, from): (usize, usize, usize), focused: bool) {
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
        let ink = match count > 1 {
            true => ink,
            false => Color32::from_gray(100),
        };
        ui.painter().text(at, align, text, font.clone(), ink);
    };
    arrow("◀", rect.left_center() + vec2(8.0, 0.0), egui::Align2::LEFT_CENTER);
    arrow("▶", rect.right_center() - vec2(8.0, 0.0), egui::Align2::RIGHT_CENTER);
    let shown = format!("{value}   {}/{}", index + from, (count + from).saturating_sub(1));
    let galley = ui.painter().layout(shown, font, ink, rect.width() - 48.0);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, ink);
}

/// The list of every value, for a mouse, with a search box when it is long. Returns the value picked.
fn list(
    ui: &mut Ui,
    id: egui::Id,
    (index, count, from): (usize, usize, usize),
    label_of: &dyn Fn(usize) -> String,
) -> Option<usize> {
    let mut picked = None;
    let key = id.with("search");
    ui.menu_button("▼", |ui| {
        let mut search: String = ui.data(|data| data.get_temp(key)).unwrap_or_default();
        if count > SEARCH_ABOVE {
            ui.add(egui::TextEdit::singleline(&mut search).hint_text("Search"));
        }
        ui.data_mut(|data| data.insert_temp(key, search.clone()));
        let search = search.trim().to_ascii_lowercase();
        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            for at in 0..count {
                let label = label_of(at);
                if at != 0 && !label.to_ascii_lowercase().contains(&search) {
                    continue;
                }
                if ui.selectable_label(at == index, format!("{}: {label}", at + from)).clicked() {
                    picked = Some(at);
                    ui.close();
                }
            }
        });
    });
    picked
}

#[cfg(test)]
mod tests {
    use super::{jump_index, next_index};

    #[test]
    fn stepping_wraps_round_the_list() {
        assert_eq!(next_index(0, 3, true), 1);
        assert_eq!(next_index(2, 3, true), 0);
        assert_eq!(next_index(0, 3, false), 2);
        assert_eq!(next_index(1, 3, false), 0);
        assert_eq!(next_index(0, 0, true), 0);
        assert_eq!(next_index(0, 1, true), 0);
    }

    #[test]
    fn a_jump_is_ten_places_and_stops_at_the_ends() {
        assert_eq!(jump_index(0, 300, true), 10);
        assert_eq!(jump_index(295, 300, true), 299);
        assert_eq!(jump_index(5, 300, false), 0);
        assert_eq!(jump_index(50, 300, false), 40);
        assert_eq!(jump_index(0, 0, true), 0);
    }
}
