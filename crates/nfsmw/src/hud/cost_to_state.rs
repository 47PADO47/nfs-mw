//! The cost-to-state flasher (`CTS_GROUP`: a title over the chase's running cost to state). Each rise of the cost
//! during a chase shows the new total for 120 HUD updates, then the group plays out; a chase that starts again at zero
//! clears the total without showing it (`docs/specs/hud-pursuit.md`, "Cost to state").

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

use crate::i18n::fmt::grouped;

/// The group, its title and its number.
pub(super) const GROUP: &str = "CTS_GROUP";
const TITLE: u32 = 0x6424_7241;
const VALUE: u32 = 0x3FF5_F33C;
/// The title's language label while the total shows (`HUD_PURSUITCTS`).
const TITLE_LABEL: u32 = 0x3DD8_74C5;
/// The group's authored scripts (600 ms each). Its `INIT` chains to `HIDE`, so it rests hidden.
const APPEAR: u32 = 0x5079_C8F8;
const LEAVE: u32 = 0x0331_13AC;
/// How long a rise keeps the total up: the game's 120 HUD updates at 60 updates a second **[inferred]**.
pub const SHOW_SECS: f32 = 2.0;
/// The leave script's length: the group stays in the layout while it plays out.
const LEAVE_SECS: f32 = 0.6;

/// The flasher's state, apart from the package.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Flasher {
    /// The highest cost to state fed since the last zero.
    total: u32,
    /// Seconds the total stays up.
    left: f32,
    /// The total is up (its appear was played).
    on: bool,
    /// Seconds of the leave script still to play.
    leaving: f32,
}

impl Flasher {
    /// The chase's cost to state this update; `None` outside a chase, which feeds nothing. A rise shows the new total
    /// for [`SHOW_SECS`]; a zero clears it without showing; anything else is ignored.
    pub fn feed(&mut self, cost: Option<u32>) {
        let Some(cost) = cost else { return };
        if cost > self.total {
            self.total = cost;
            self.left = SHOW_SECS;
            return;
        }
        if cost == 0 {
            self.total = 0;
        }
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
pub struct CostToState {
    group: Option<ObjectRef>,
    title: Option<ObjectRef>,
    value: Option<ObjectRef>,
    flasher: Flasher,
}

impl CostToState {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        Self {
            group: rt.find(package, fe_hash_upper(GROUP)),
            title: rt.find(package, TITLE),
            value: rt.find(package, VALUE),
            flasher: Flasher::default(),
        }
    }

    /// Feeds this update's cost (`None` outside a chase) and advances `dt` seconds; returns the script to play. Call it
    /// before the layout is decided, which takes [`CostToState::shown`].
    pub fn advance(&mut self, cost: Option<u32>, dt: f32) -> Option<u32> {
        self.flasher.feed(cost);
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
            rt.set_text(value, grouped(i64::from(self.flasher.total)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A quarter second: two seconds are exactly eight updates.
    const STEP: f32 = 0.25;
    const FRAME: f32 = 1.0 / 60.0;

    #[test]
    fn a_rise_shows_the_total_for_two_seconds_then_plays_out() {
        let mut flasher = Flasher::default();
        flasher.feed(Some(2000));
        assert_eq!(flasher.step(STEP), Some(APPEAR));
        assert!(flasher.shown());
        for _ in 0..7 {
            assert_eq!(flasher.step(STEP), None, "up for two seconds");
        }
        assert_eq!(flasher.step(STEP), Some(LEAVE));
        assert!(flasher.shown(), "in the layout while the leave plays");
        flasher.step(STEP);
        flasher.step(STEP);
        assert!(flasher.shown());
        flasher.step(STEP);
        assert!(!flasher.shown(), "the leave has played");
    }

    #[test]
    fn a_further_rise_restarts_the_time_without_a_second_appear() {
        let mut flasher = Flasher::default();
        flasher.feed(Some(500));
        assert_eq!(flasher.step(1.5), Some(APPEAR));
        flasher.feed(Some(750));
        assert_eq!(flasher.step(1.5), None, "still up: no new appear");
        assert_eq!(flasher.step(0.4), None, "the rise restarted the two seconds");
        assert_eq!(flasher.total, 750);
    }

    #[test]
    fn the_same_cost_or_no_chase_shows_nothing_and_a_zero_clears_the_total() {
        let mut flasher = Flasher { total: 4000, ..Flasher::default() };
        flasher.feed(Some(4000));
        flasher.feed(None);
        assert_eq!(flasher.step(FRAME), None);
        flasher.feed(Some(0));
        assert_eq!(flasher.total, 0, "a new chase starts from zero");
        assert_eq!(flasher.step(FRAME), None, "the clear is not shown");
        flasher.feed(Some(250));
        assert_eq!(flasher.step(FRAME), Some(APPEAR), "the next chase's first cost shows");
    }
}
