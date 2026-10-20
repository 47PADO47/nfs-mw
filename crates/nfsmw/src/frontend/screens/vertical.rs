//! The stack's vertical menus: the list a screen publishes with its footer hints, the row under the pointer, and the pad
//! a vertical menu sees (`ui/theme/remastered/menu`).

use glam::Vec2;

use super::{Env, Pick, Screen, Screens};
use crate::frontend::logic::Way;
use crate::ui::theme::remastered::menu::layout as menu_layout;
use crate::ui::theme::{MenuHint, MenuList};

impl Screens {
    /// The screen's list with the footer hints and the window's aspect filled in.
    pub(super) fn list_of(
        &self,
        screen: &Screen,
        bindings: &crate::input::Bindings,
        device: crate::input::InputDevice,
    ) -> Option<MenuList> {
        let mut list = screen.logic.list(&self.assets)?;
        list.hints = crate::frontend::prompts::hint_set(&screen.name, bindings, device)
            .into_iter()
            .map(|(keys, label, action)| MenuHint { keys, label, action })
            .collect();
        list.aspect = self.aspect;
        Some(list)
    }

    /// The entry of the top screen's vertical list under a point, when the top screen is one.
    pub(super) fn row_at(&self, point: Vec2) -> Option<usize> {
        let top = self.stack.last().filter(|top| top.logic.vertical())?;
        let list = top.logic.list(&self.assets)?;
        let shown = menu_layout::shown(list.entries.len().saturating_sub(list.first));
        menu_layout::row_at(point, shown, self.aspect, list.has_values()).map(|row| list.first + row)
    }

    /// Which way a click at `point` presses a row of the top screen's list: back on the left chevron, forward on the right.
    pub(super) fn way_at(&self, point: Vec2) -> Way {
        match menu_layout::side_at(point.x - menu_layout::left_edge(self.aspect)) {
            menu_layout::Side::Left => Way::Back,
            menu_layout::Side::Right => Way::Forward,
            menu_layout::Side::Middle => Way::Stay,
        }
    }

    /// Whether the top screen is a vertical list: the package's own icons are not on screen then, so they take no clicks.
    pub(super) fn top_is_vertical(&self) -> bool {
        self.stack.last().is_some_and(|top| top.logic.vertical())
    }

    /// The pointer moved to `point` (`None` when it left the window): a vertical menu selects the row under it. A pointer
    /// that has not moved changes nothing, so the keys and the pad keep the selection.
    pub fn hover_row(&mut self, point: Option<Vec2>, env: &mut Env) {
        if std::mem::replace(&mut self.pointer, point) == point {
            return;
        }
        let Some(row) = point.and_then(|p| self.row_at(p)) else { return };
        self.on_top(env, |logic, cx| {
            logic.pick_row(cx, row, Pick::Hover);
            true
        });
    }
}

/// The pad a vertical menu sees: Up and Down do what Left and Right do on the icon row, and Left and Right nothing. The
/// runtime then repeats a held direction and plays the menu's sounds as it always did.
pub(super) fn vertical_pad(mask: u32) -> u32 {
    use blackbox_feng::runtime::pad;
    let mut out = mask & !(pad::UP | pad::DOWN | pad::LEFT | pad::RIGHT);
    if mask & pad::UP != 0 {
        out |= pad::LEFT;
    }
    if mask & pad::DOWN != 0 {
        out |= pad::RIGHT;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::vertical_pad;
    use blackbox_feng::runtime::pad;

    #[test]
    fn up_and_down_become_left_and_right_and_the_sideways_keys_do_nothing() {
        assert_eq!(vertical_pad(pad::UP), pad::LEFT);
        assert_eq!(vertical_pad(pad::DOWN), pad::RIGHT);
        assert_eq!(vertical_pad(pad::LEFT | pad::RIGHT), 0, "a vertical list has no sideways");
        assert_eq!(vertical_pad(pad::ACCEPT | pad::DOWN), pad::ACCEPT | pad::RIGHT, "the other buttons pass");
    }
}
