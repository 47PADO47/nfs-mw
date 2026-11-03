//! The installed sheets have nine EVENT_ICON slots, three columns by three rows. Keep their authored layout and scripts.

use blackbox_feng::ids::{PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_UP};
use blackbox_feng::{ObjectRef, fe_hash_upper};
use glam::Vec2;

use super::{ICON_GRID, Page, Status};
use crate::frontend::logic::Cx;

const WIDTH: usize = 3;
const HEIGHT: usize = 3;
const SLOTS: usize = WIDTH * HEIGHT;
const INIT: u32 = 0x0017_44B3;
const HIGHLIGHT: u32 = 0x249D_B7B7;
const UNHIGHLIGHT: u32 = 0x7AB5_521A;
const BACKING: u32 = 0x688A_1551;
const HANDLE: u32 = 0x3FBB_7A4E;
const DETAIL_ICON: u32 = 0xF97E_C5D5;
const LOCK: u32 = 0x0018_ED48;
const CHECK: u32 = 0x028F_EADD;

/// Left/Right wrap within the current row; Up/Down move by one row and stop at the top/bottom.
pub(super) fn step(at: usize, len: usize, message: u32) -> usize {
    if len == 0 {
        return 0;
    }
    let at = at.min(len - 1);
    let first = at / WIDTH * WIDTH;
    let columns = (len - first).min(WIDTH);
    match message {
        PAD_LEFT => first + (at - first + columns - 1) % columns,
        PAD_RIGHT => first + (at - first + 1) % columns,
        PAD_UP => at.checked_sub(WIDTH).unwrap_or(at),
        PAD_DOWN if first + WIDTH < len => (at + WIDTH).min(len - 1),
        _ => at,
    }
}

#[derive(Default)]
pub(super) struct Icons {
    first: usize,
    selected: [Option<bool>; SLOTS],
}

impl Icons {
    pub fn reset(&mut self) {
        self.selected = [None; SLOTS];
    }

    pub fn refresh(&mut self, cx: &mut Cx, page: &dyn Page, at: usize, visible: bool) {
        cx.hide(ICON_GRID, !visible);
        if at < self.first {
            self.first = at / WIDTH * WIDTH;
        }
        if at >= self.first + SLOTS {
            self.first = (at / WIDTH + 1).saturating_sub(HEIGHT) * WIDTH;
        }
        for slot in 0..SLOTS {
            let name = fe_hash_upper(&format!("EVENT_ICON_{}", slot + 1));
            let Some(icon) = cx.object(name) else { continue };
            let index = self.first + slot;
            let shown = index < page.len();
            // Hide the slot's vignette along with its icon; the state marker may be outside that holder.
            if let Some(parent) =
                cx.rt.package(cx.package).and_then(|p| p.objects[icon.index].parent).and_then(|guid| cx.guid(guid))
            {
                cx.rt.set_hidden(parent, !shown);
            }
            self.marker(cx, slot, shown.then(|| page.status(index)));
            if !shown {
                self.selected[slot] = None;
                continue;
            }
            if let Some(texture) = page.icon(index) {
                cx.rt.set_texture(icon, texture);
            }
            let selected = index == at;
            if self.selected[slot] == Some(selected) {
                continue;
            }
            let script = match (selected, self.selected[slot]) {
                (true, _) => HIGHLIGHT,
                (false, Some(true)) => UNHIGHLIGHT,
                _ => INIT,
            };
            cx.rt.run_script(icon, script);
            self.selected[slot] = Some(selected);
        }
        if visible
            && at < page.len()
            && let Some(icon) = cx.object(fe_hash_upper(&format!("EVENT_ICON_{}", at - self.first + 1)))
            && cx.rt.current_button(cx.package) != Some(icon)
        {
            cx.rt.focus_button(icon);
        }
        if let Some(icon) = cx.object(DETAIL_ICON) {
            cx.rt.set_hidden(icon, page.icon(at).is_none());
            if let Some(texture) = page.icon(at) {
                cx.rt.set_texture(icon, texture);
            }
        }
        self.scrollbar(cx, page.len(), visible);
    }

    fn marker(&self, cx: &mut Cx, slot: usize, status: Option<Status>) {
        // Race/milestones use the separate Performance_Details placeholders; bounty has per-holder CHECK images.
        for name in [0x7AD9_3591 + slot as u32, 0xE636_228D + slot as u32] {
            let Some(marker) = cx.object(name) else { continue };
            let texture = match status {
                Some(Status::Locked) => Some(LOCK),
                Some(Status::Done) => Some(CHECK),
                _ => None,
            };
            cx.rt.set_hidden(marker, texture.is_none());
            if let Some(texture) = texture {
                cx.rt.set_texture(marker, texture);
            }
        }
    }

    fn scrollbar(&self, cx: &mut Cx, len: usize, visible: bool) {
        let rows = len.div_ceil(WIDTH);
        let shown = visible && rows > HEIGHT;
        let (Some(backing), Some(handle)) = (cx.object(BACKING), cx.object(HANDLE)) else { return };
        cx.rt.set_hidden(backing, !shown);
        cx.rt.set_hidden(handle, !shown);
        if !shown {
            return;
        }
        let (Some(at), Some(size), Some(handle_size)) =
            (cx.rt.position(backing), cx.rt.size(backing), cx.rt.size(handle))
        else {
            return;
        };
        let height = size.y * HEIGHT as f32 / rows as f32;
        let top = at.y - size.y * 0.5;
        let travel = (size.y - height) / (rows - HEIGHT) as f32;
        cx.rt.set_size_xy(handle, handle_size.x, height);
        cx.rt.set_position_xy(handle, at.x, top + self.first as f32 / WIDTH as f32 * travel + height * 0.5);
    }

    pub fn hit(&self, cx: &Cx, point: Vec2, len: usize) -> Option<usize> {
        let icons: Vec<Option<ObjectRef>> =
            (1..=SLOTS).map(|slot| cx.object(fe_hash_upper(&format!("EVENT_ICON_{slot}")))).collect();
        let slot = cx.hit_index(&icons, point)?;
        (self.first + slot < len).then_some(self.first + slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_navigation_handles_partial_rows_and_stops_at_vertical_ends() {
        assert_eq!(step(0, 13, PAD_UP), 0);
        assert_eq!(step(0, 13, PAD_DOWN), 3);
        assert_eq!(step(0, 13, PAD_LEFT), 2);
        assert_eq!(step(2, 13, PAD_RIGHT), 0);
        assert_eq!(step(11, 13, PAD_DOWN), 12);
        assert_eq!(step(12, 13, PAD_RIGHT), 12);
        assert_eq!(step(12, 13, PAD_DOWN), 12);
        assert_eq!(step(0, 0, PAD_DOWN), 0);
    }
}
