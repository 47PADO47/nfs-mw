//! Installed sheet artwork on the overview's authored OPTION_n slots, replacing its car-shaped stand-in. The row is the
//! shared icon strip (`icon_strip.rs`): it scrolls, fades and takes clicks as the main menu's row does.

use blackbox_feng::fe_hash_upper;
use glam::Vec2;

use super::{SHEETS, Sheet};
use crate::frontend::icon_strip::{Face, Hit, IconStrip, WHITE};
use crate::frontend::logic::Cx;

pub fn texture(sheet: Sheet) -> u32 {
    fe_hash_upper(match sheet {
        Sheet::RaceEvents => "MAIN_ICON_REP_SHEET",
        Sheet::Milestones => "MILESTONE_MAIN",
        Sheet::Bounty => "MAIN_ICON_BOUNTY",
        Sheet::Bio => "MAIN_ICON_RIVALBIO",
    })
}

pub struct Row {
    strip: IconStrip,
}

impl Default for Row {
    fn default() -> Self {
        let faces = SHEETS.iter().map(|&sheet| Face::new(texture(sheet), sheet.screen().is_some())).collect();
        Self { strip: IconStrip::new(faces, WHITE) }
    }
}

impl Row {
    /// Finds the package's slots and shows the row at `sheet`.
    pub fn start(&mut self, cx: &mut Cx, sheet: usize) {
        self.strip.build(cx);
        let first = self.strip.first();
        self.strip.open(cx, first + sheet);
    }

    /// The row eases to `sheet`.
    pub fn select(&mut self, cx: &mut Cx, sheet: usize) {
        let entry = self.strip.first() + sheet;
        self.strip.select(cx, entry);
    }

    /// The package began to leave: the row shrinks out with it.
    pub fn leave(&mut self) {
        self.strip.leave();
    }

    pub fn tick(&mut self, cx: &mut Cx, dt: f32) {
        self.strip.tick(dt);
        self.strip.place(cx);
    }

    /// The sheet whose icon lies under `point`, and whether the point is on the row at all.
    pub fn hit(&self, cx: &Cx, point: Vec2) -> Option<Hit> {
        self.strip.hit(cx, point)
    }
}
