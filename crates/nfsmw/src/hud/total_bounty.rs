//! The career's TOTAL BOUNTY flasher (`docs/specs/hud-pursuit.md`, "Total bounty"): a title over the career's total
//! bounty on an info panel, beside the cost-to-state flasher. The HUD is given the total when a chase ends; a total
//! other than the one it holds shows for 120 HUD updates, then the group plays out. Career mode only.

use blackbox_feng::{ObjectRef, PackageId, Runtime};

use super::cost_to_state::SHOW_SECS;
use crate::i18n::fmt::grouped;

/// The group, its title and its number.
pub(super) const GROUP: u32 = 0xEA90_3012;
const TITLE: u32 = 0x41A5_5ECF;
const VALUE: u32 = 0x9B0A_C8CA;
/// The title's language label while the total shows (`HUD_TOTALBOUNTY`).
const TITLE_LABEL: u32 = 0x7D01_71E4;
/// The group's authored scripts (600 ms each). Its `INIT` chains to `HIDE`, so it rests hidden.
pub(super) const APPEAR: u32 = 0x5079_C8F8;
const LEAVE: u32 = 0x0331_13AC;
/// The leave script's length: the group stays in the layout while it plays out.
const LEAVE_SECS: f32 = 0.6;

/// The flasher's state, apart from the package.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Flasher {
    /// The last total given; 0 before the first.
    held: u32,
    /// Seconds the total stays up.
    left: f32,
    /// The total is up (its appear was played).
    on: bool,
    /// Seconds of the leave script still to play.
    leaving: f32,
}

impl Flasher {
    /// The career's total bounty as the last chase's end gave it (`None` before one, and outside the career). A total
    /// other than the one held is held and shown for [`SHOW_SECS`]; the same total shows nothing.
    pub fn feed(&mut self, total: Option<u32>) {
        let Some(total) = total else { return };
        if total == self.held {
            return;
        }
        self.held = total;
        self.left = SHOW_SECS;
    }

    /// One HUD update of `dt` seconds: the script the group should start, if any.
    pub fn step(&mut self, dt: f32) -> Option<u32> {
        self.leaving = (self.leaving - dt).max(0.0);
        if self.left > 0.0 {
            self.left -= dt;
            if self.on {
                return None;
            }
            self.on = true;
            return Some(APPEAR);
        }
        if !self.on {
            return None;
        }
        self.on = false;
        self.leaving = LEAVE_SECS;
        Some(LEAVE)
    }

    /// The group belongs in the layout: the total is up or its leave plays.
    pub fn shown(&self) -> bool {
        self.on || self.leaving > 0.0
    }
}

/// The flasher's objects in the package.
pub struct TotalBounty {
    group: Option<ObjectRef>,
    title: Option<ObjectRef>,
    value: Option<ObjectRef>,
    flasher: Flasher,
}

impl TotalBounty {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        Self {
            group: rt.find(package, GROUP),
            title: rt.find(package, TITLE),
            value: rt.find(package, VALUE),
            flasher: Flasher::default(),
        }
    }

    /// Feeds this update's total (`None` outside the career) and advances `dt` seconds; returns the script to play.
    /// Call it before the layout is decided, which takes [`TotalBounty::shown`].
    pub fn advance(&mut self, total: Option<u32>, dt: f32) -> Option<u32> {
        self.flasher.feed(total);
        self.flasher.step(dt)
    }

    pub fn shown(&self) -> bool {
        self.flasher.shown()
    }

    /// Plays `script` and, while the total is up, writes the title and the total. Call it after the layout update.
    pub fn apply(&self, rt: &mut Runtime, script: Option<u32>) {
        let Some(group) = self.group else { return };
        if let Some(script) = script {
            rt.run_script(group, script);
        }
        if !self.flasher.on {
            return;
        }
        if let Some(title) = self.title {
            rt.set_label(title, TITLE_LABEL);
        }
        if let Some(value) = self.value {
            rt.set_text(value, grouped(i64::from(self.flasher.held)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A quarter second: two seconds are exactly eight updates.
    const STEP: f32 = 0.25;

    #[test]
    fn a_new_total_shows_for_two_seconds_then_plays_out() {
        let mut flasher = Flasher::default();
        flasher.feed(Some(125_000));
        assert_eq!(flasher.step(STEP), Some(APPEAR));
        for _ in 0..7 {
            assert_eq!(flasher.step(STEP), None, "up for two seconds");
        }
        assert_eq!(flasher.step(STEP), Some(LEAVE));
        assert!(flasher.shown(), "in the layout while the leave plays");
        for _ in 0..3 {
            flasher.step(STEP);
        }
        assert!(!flasher.shown(), "the leave has played");
    }

    #[test]
    fn the_same_total_or_none_shows_nothing_and_a_lower_one_shows() {
        let mut flasher = Flasher::default();
        flasher.feed(None);
        flasher.feed(Some(0));
        assert_eq!(flasher.step(STEP), None, "nothing given yet, or the zero it starts from");
        flasher.feed(Some(40_000));
        flasher.step(3.0);
        flasher.step(1.0);
        flasher.feed(Some(40_000));
        assert_eq!(flasher.step(STEP), None, "the total the last chase left does not show again");
        flasher.feed(Some(30_000));
        assert_eq!(flasher.step(STEP), Some(APPEAR), "any change shows, a fall too");
        assert_eq!(flasher.held, 30_000);
    }
}
