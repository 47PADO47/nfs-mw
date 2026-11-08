//! How the selection moves through the option page and what a press does (docs/specs/menu-sections.md, section 3). The
//! rows include section titles, which the selection never lands on: the arrows, the d-pad, the stick, the wheel, the page
//! jumps and the pointer's hover all go through `blackbox_ui::menu::nav`, which passes them over, and a click on one does
//! nothing.

use blackbox_ui::menu::EntryFlags;
use blackbox_ui::menu::nav;
use glam::{Mat4, Vec2, Vec3};

use super::{Command, Cx, WidgetMenu, mod_options, options};
use blackbox_feng::Runtime;

/// How many boxes below the middle of the object's box `point` is: the point's y in the box's own axes. The box's matrix may
/// have no depth (a string's), so it is solved over the screen's two axes and not inverted. NaN for a box with no area.
fn box_row(quad: &Mat4, point: Vec2) -> f32 {
    let (ex, ey) = (quad.x_axis.truncate().truncate(), quad.y_axis.truncate().truncate());
    let d = point - quad.w_axis.truncate().truncate();
    let det = ex.x * ey.y - ex.y * ey.x;
    if det.abs() < f32::EPSILON {
        return f32::NAN;
    }
    (ex.x * d.y - ex.y * d.x) / det
}

impl WidgetMenu {
    /// Where the page opens: the first row the selection can stop on, so a page whose first row is a title opens on the
    /// row under it.
    pub(super) fn first_selection(&self) -> usize {
        nav::first_stop(&self.flags).unwrap_or(0)
    }

    /// The row a title row sits directly above: the window shows that title with the row, so a section is never entered
    /// without its heading in view. The row itself when no title is above it.
    pub(super) fn with_title(&self, index: usize) -> usize {
        match index.checked_sub(1).and_then(|above| self.flags.get(above)) {
            Some(flags) if !flags.highlightable => index - 1,
            _ => index,
        }
    }

    /// Selects the previous or next row the selection can stop on, wrapping round, and scrolls the list when it must.
    pub(super) fn move_selection(&mut self, cx: &mut Cx, forward: bool) {
        if self.rows.is_empty() || self.window() == 0 || self.leaving {
            return;
        }
        let Some(next) = nav::step(&self.flags, self.selected, forward) else { return };
        self.go_to(cx, next);
    }

    /// A page jump (the shoulder buttons): a window's worth of rows down or up, on the nearest row that can be selected.
    pub(super) fn page(&mut self, cx: &mut Cx, forward: bool) {
        if self.rows.is_empty() || self.window() == 0 || self.leaving {
            return;
        }
        let Some(next) = nav::page(&self.flags, self.selected, self.window(), forward) else { return };
        self.go_to(cx, next);
    }

    /// Selects the row `index` (one the selection can be on) and scrolls the window to keep it, and the title above it, in
    /// view.
    fn go_to(&mut self, cx: &mut Cx, index: usize) {
        self.selected = index;
        let title = self.with_title(index);
        if title < self.top {
            self.top = title;
        }
        if self.selected >= self.top + self.window() {
            self.top = self.selected + 1 - self.window();
        }
        self.layout_rows(cx);
    }

    /// The visible row under a point, as its slot on screen. A row runs across the title and the data columns.
    /// The title master's box is one row high, and the row in slot `k` lies `k` boxes further down.
    fn slot_at(&self, rt: &Runtime, point: Vec2) -> Option<usize> {
        let layout = self.layout.as_ref()?;
        let title = rt.object_quad(layout.title_object)?;
        let data = rt.object_quad(layout.data_object)?;
        let edges =
            [title.transform_point3(Vec3::new(-0.5, 0.0, 0.0)).x, data.transform_point3(Vec3::new(0.5, 0.0, 0.0)).x];
        let (left, right) = (edges[0].min(edges[1]), edges[0].max(edges[1]));
        if point.x < left || point.x > right {
            return None;
        }
        let row = (box_row(&title, point) + 0.5).floor();
        if row < 0.0 || row.is_nan() {
            return None;
        }
        let slot = row as usize;
        (self.top + slot < (self.top + self.window()).min(self.rows.len())).then_some(slot)
    }

    /// Selects a row, as the pointer does. A row that cannot be highlighted (a section title) is left alone.
    pub(super) fn select(&mut self, cx: &mut Cx, index: usize) {
        if self.leaving || self.selected == index || !nav::can_hover(&self.flags, index) {
            return;
        }
        self.go_to(cx, index);
    }

    /// The flags of the selected row.
    fn selected_flags(&self) -> EntryFlags {
        self.flags.get(self.selected).copied().unwrap_or(EntryFlags::ROW)
    }

    /// A click on a row selects it; a click on one of its arrows then changes the setting in that direction. A click on a
    /// section title does nothing, and nothing else reacts to it either.
    pub(super) fn click_row(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        if self.leaving {
            return false;
        }
        let Some(slot) = self.slot_at(cx.rt, point) else { return false };
        if !nav::can_hover(&self.flags, self.top + slot) {
            return true;
        }
        let Some((left, right)) = self.objects.get(slot).map(|r| (r.left, r.right)) else { return false };
        let arrow = cx.hit_index(&[left, right], point);
        self.select(cx, self.top + slot);
        if self.rows[self.selected].setting == options::Setting::AdvancedGraphics {
            self.change_from_pointer(cx, true);
            return true;
        }
        if let Some(arrow) = arrow {
            self.change_from_pointer(cx, arrow == 1);
        }
        true
    }

    /// Left or right on the selected row changes its setting; the new value is in effect at once.
    pub(super) fn change(&mut self, cx: &mut Cx, forward: bool) {
        let Some(row) = self.rows.get(self.selected).copied() else { return };
        if self.leaving || !self.selected_flags().clickable {
            return;
        }
        self.nudge = Some((self.selected, forward, self.clock));
        if row.setting == options::Setting::AdvancedGraphics {
            cx.send(Command::OpenGraphics);
            return;
        }
        if let options::Setting::Mod(_) | options::Setting::ModManaged(_) | options::Setting::GameplayOption(_) =
            row.setting
        {
            mod_options::change(cx.mods, row.setting, forward);
            self.draw_visible(cx);
            return;
        }
        if row.setting == options::Setting::Autosave {
            options::autosave::change(cx);
            self.draw_visible(cx);
            return;
        }
        row.setting.advance(cx.settings, cx.changed, forward, &cx.caps);
        self.draw_visible(cx);
    }
}
