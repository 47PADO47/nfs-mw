//! The key list's values (`ControlsList`): a control's two bindings as two key caps, one per half of the row's value
//! column, the primary on the left. The selected row's focused cap is raised, with an accent line under it; an unbound
//! key (the frontend's `---`) reads faint. The halves are the ones the frontend tests a click against (a press left of
//! the column's middle picks the primary binding), so the caps sit where a click lands.

use blackbox_ui::{LayoutCx, Rect, Rounded};

use super::list::{DISABLED, fitted_size};
use super::{Align, Parts};
use crate::ui::theme::custom::text::Label;
use crate::ui::theme::custom::tokens::{faded_skin, keys};

/// The space between the two caps, a cap's height and corner, the accent line's thickness, and the key name's size, at
/// scale 1.
const CAP_GAP: f32 = 8.0;
const CAP_HEIGHT: f32 = 20.0;
const CAP_RADIUS: f32 = 2.0;
const UNDERLINE: f32 = 2.0;
const KEY_SIZE: f32 = 13.0;
/// The room a key name keeps from its cap's edges, at scale 1.
const CAP_PAD: f32 = 6.0;
/// An idle cap's fill alpha.
const IDLE_CAP: f32 = 0.6;

/// Whether a key column shows no binding: the frontend's placeholder is a run of dashes.
pub(super) fn unbound(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c == '-')
}

/// The two caps in `column` at `scale`, the primary first: each takes half the column less the gap between them.
pub(super) fn cap_rects(column: Rect, scale: f32) -> [Rect; 2] {
    let gap = CAP_GAP * scale;
    let width = ((column.w - gap) / 2.0).max(0.0);
    let height = (CAP_HEIGHT * scale).min(column.h);
    [
        Rect::new(column.min_x() + width / 2.0, column.y, width, height),
        Rect::new(column.max_x() - width / 2.0, column.y, width, height),
    ]
}

/// How a cap is drawn: its fill key and alpha, its text colour key and alpha.
fn cap_look(enabled: bool, lit: bool, unbound: bool) -> (&'static str, f32, &'static str, f32) {
    match (enabled, lit, unbound) {
        (false, ..) => (keys::RULE, IDLE_CAP * DISABLED, keys::TEXT_SECONDARY, DISABLED),
        (true, true, _) => (keys::SURFACE_RAISED, 1.0, keys::TEXT, 1.0),
        (true, false, true) => (keys::RULE, IDLE_CAP, keys::TEXT_SECONDARY, DISABLED),
        (true, false, false) => (keys::RULE, IDLE_CAP, keys::TEXT_SECONDARY, 1.0),
    }
}

impl Parts<'_, '_> {
    /// The two bindings of a control in `column`. `focus` is the cap that has the focus, on the selected row only.
    pub(super) fn key_caps(&mut self, names: [&str; 2], column: Rect, enabled: bool, focus: Option<usize>) {
        let frame = self.frame();
        let s = frame.scale();
        for (slot, (name, cap)) in names.into_iter().zip(cap_rects(column, s)).enumerate() {
            let lit = enabled && focus == Some(slot);
            let (fill, fill_alpha, colour, alpha) = cap_look(enabled, lit, unbound(name));
            let skin = faded_skin(self.cx().skin, &[fill], &[], fill_alpha);
            let plate = Rounded::new(fill, cap, CAP_RADIUS * s, &LayoutCx::new(&skin, self.cx().time_s));
            self.add(plate);
            if lit {
                let line = Rect::new(cap.x, cap.max_y() - UNDERLINE * s / 2.0, cap.w, UNDERLINE * s);
                self.panel(keys::ACCENT, line, 1.0);
            }
            let room = (cap.w - 2.0 * CAP_PAD * s).max(0.0);
            let size = fitted_size(name, KEY_SIZE, room, frame.sizing());
            let label = Label::label(name.to_owned(), size).colour(colour).opacity(alpha);
            self.text(label, (cap.x, cap.y), Align::Centre);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_caps_split_the_column_at_its_middle() {
        let column = Rect::new(100.0, 20.0, 200.0, 24.0);
        let [primary, secondary] = cap_rects(column, 1.0);
        assert!(primary.max_x() < column.x && secondary.min_x() > column.x, "each cap keeps to its half");
        assert!((primary.min_x() - column.min_x()).abs() < 1e-4 && (secondary.max_x() - column.max_x()).abs() < 1e-4);
        assert!((secondary.min_x() - primary.max_x() - CAP_GAP).abs() < 1e-4);
        assert!(primary.h <= column.h);
    }

    #[test]
    fn a_run_of_dashes_is_an_unbound_key() {
        assert!(unbound("---"));
        assert!(!unbound("W") && !unbound("") && !unbound("Num -"));
    }

    #[test]
    fn only_the_focused_cap_of_an_enabled_row_is_raised() {
        assert_eq!(cap_look(true, true, false).0, keys::SURFACE_RAISED);
        assert_eq!(cap_look(true, false, false).0, keys::RULE);
        assert_eq!(cap_look(false, false, false).3, DISABLED);
        assert!(cap_look(true, false, true).3 < cap_look(true, false, false).3, "an unbound key is fainter");
    }
}
