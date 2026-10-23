//! Installed sheet artwork on the overview's authored OPTION_n slots, replacing its car-shaped stand-in.

use blackbox_feng::{ObjectRef, fe_hash_upper};
use glam::Vec2;

use super::super::blacklist::DISABLED_ALPHA;
use super::{SHEETS, Sheet};
use crate::frontend::ids;
use crate::frontend::logic::Cx;
use crate::frontend::scroller::{self, Fade};

const BOOK_ENDS: usize = 2;
const SIZE: f32 = 64.0;
const PITCH: f32 = SIZE + scroller::SPACING;

pub fn texture(sheet: Sheet) -> u32 {
    fe_hash_upper(match sheet {
        Sheet::RaceEvents => "MAIN_ICON_REP_SHEET",
        Sheet::Milestones => "MILESTONE_MAIN",
        Sheet::Bounty => "MAIN_ICON_BOUNTY",
        Sheet::Bio => "MAIN_ICON_RIVALBIO",
    })
}

#[derive(Default)]
pub struct Row {
    slots: Vec<ObjectRef>,
    center: Vec2,
    fade: Fade,
}

impl Row {
    pub fn start(&mut self, cx: &mut Cx) {
        self.slots = (1..).map_while(|n| cx.named(&format!("OPTION_{n}"))).collect();
        if let Some(at) = cx.object(ids::OPTION_MASTER).and_then(|o| cx.rt.position(o)) {
            self.center = at.truncate();
        }
    }

    pub fn tick(&mut self, cx: &mut Cx, selected: usize, dt: f32) {
        self.fade.update(dt);
        self.place(cx, selected);
    }

    pub fn place(&self, cx: &mut Cx, selected: usize) {
        cx.hide(ids::OPTION_MASTER, true);
        for (index, object) in self.slots.iter().copied().enumerate() {
            let sheet = index.checked_sub(BOOK_ENDS).and_then(|n| SHEETS.get(n)).copied();
            let x = self.center.x + (index as f32 - (selected + BOOK_ENDS) as f32) * PITCH;
            let scale = scroller::scale(x, self.center.x, scroller::WIDTH);
            let shown = scroller::pulled(x, self.center.x, SIZE, scale);
            let alpha = (255.0 * scale * self.fade.value()) as u8;
            let enabled = sheet.is_none_or(|sheet| sheet.screen().is_some());
            cx.rt.set_hidden(object, scale <= 0.0);
            cx.rt.set_texture(object, sheet.map_or(ids::END_OF_SCROLLER, texture));
            cx.rt.set_position_xy(object, shown, self.center.y);
            cx.rt.set_size_xy(object, SIZE * scale * self.fade.value(), SIZE * scale * self.fade.value());
            let rgb = if enabled { 255 } else { 150 };
            cx.rt.set_colour_rgba(object, [rgb, rgb, rgb, if enabled { alpha } else { alpha.min(DISABLED_ALPHA) }]);
        }
    }

    pub fn hit(&self, cx: &Cx, point: Vec2) -> Option<usize> {
        let object = cx.rt.object_at(&self.slots, point)?;
        let slot = self.slots.iter().position(|slot| *slot == object)?;
        let sheet = slot.checked_sub(BOOK_ENDS)?;
        (sheet < SHEETS.len()).then_some(sheet)
    }
}
