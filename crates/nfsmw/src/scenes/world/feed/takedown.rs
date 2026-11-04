//! The takedown line of the HUD's generic message (`hud/takedown.rs`): watches the chase's latest wrecked cop car from
//! frame to frame and holds the line while it plays. The original tells a wreck when the chase's wreck count rises
//! above the count it saw last; a lower count (a new chase) is only taken as the new baseline, and two wrecks between
//! two looks make one line, of the latest car (`docs/specs/hud-pursuit-takedown.md`).

use crate::hud::Message;
use crate::scenes::world::ai::Takedown;

/// Seconds the line is up: the length of its script.
pub const LINE_SECS: f32 = 2.4;
/// The first serial of the takedown lines, apart from the race's lines (which share the message group).
const SERIAL_BASE: u32 = 0x6000_0000;

/// The takedown line and what it was diffed from.
#[derive(Debug, Default)]
pub(in crate::scenes::world) struct TakedownLine {
    /// The wreck count of the last look; `None` before the first.
    count: Option<u32>,
    /// Lines told so far: each one takes a new serial, so a new chase's first wreck plays again.
    told: u32,
    line: Option<Message>,
    left: f32,
}

impl TakedownLine {
    /// The frame's look at the chase, `dt` seconds after the last: `latest` is the chase's latest wreck (`None` before
    /// the first). A wreck count above the last look's puts the line up.
    pub fn observe(&mut self, latest: Option<&Takedown>, dt: f32) {
        self.observe_with(latest, dt, crate::hud::takedown::message);
    }

    /// [`Self::observe`] with the line made by `line_of(serial, car, bounty)`.
    fn observe_with(
        &mut self,
        latest: Option<&Takedown>,
        dt: f32,
        line_of: impl Fn(u32, &str, u32) -> Option<Message>,
    ) {
        if dt.is_finite() && dt > 0.0 {
            self.left = (self.left - dt).max(0.0);
        }
        if self.left <= 0.0 {
            self.line = None;
        }
        let count = latest.map_or(0, |takedown| takedown.count);
        let Some(before) = self.count.replace(count) else { return };
        if count <= before {
            return;
        }
        let Some(takedown) = latest else { return };
        let serial = SERIAL_BASE.wrapping_add(self.told);
        let Some(message) = line_of(serial, &takedown.car, takedown.bounty()) else { return };
        self.told = self.told.wrapping_add(1);
        self.line = Some(message);
        self.left = LINE_SECS;
    }

    /// The line while it plays.
    pub fn message(&self) -> Option<&Message> {
        self.line.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wreck(count: u32, car: &str) -> Takedown {
        Takedown { count, car: car.to_owned(), points: 250, multiplier: 2 }
    }

    /// The line without the language table: the car's name and the bounty.
    fn plain(serial: u32, car: &str, bounty: u32) -> Option<Message> {
        crate::hud::takedown::label_of(car).map(|_| Message::takedown(serial, format!("{car} {bounty}")))
    }

    #[test]
    fn a_higher_wreck_count_puts_the_line_up_for_its_script_and_then_it_goes() {
        let mut line = TakedownLine::default();
        line.observe_with(None, 0.0, plain);
        assert!(line.message().is_none(), "the first look is the baseline");
        line.observe_with(Some(&wreck(1, "copmidsize")), 0.016, plain);
        let shown = line.message().cloned().expect("the wreck is told");
        assert_eq!(shown.text, "copmidsize 500", "the points times the multiplier");
        assert_eq!(shown.kind, crate::hud::MessageKind::Takedown);
        line.observe_with(Some(&wreck(1, "copmidsize")), 1.0, plain);
        assert_eq!(line.message(), Some(&shown), "the same count keeps the line while it plays");
        line.observe_with(Some(&wreck(1, "copmidsize")), LINE_SECS, plain);
        assert!(line.message().is_none(), "gone when its script has played");
    }

    #[test]
    fn a_new_chases_first_wreck_takes_a_new_serial() {
        let mut line = TakedownLine::default();
        line.observe_with(None, 0.0, plain);
        line.observe_with(Some(&wreck(1, "copmidsize")), 0.016, plain);
        let first = line.message().map(|m| m.serial);
        line.observe_with(None, 0.016, plain);
        line.observe_with(Some(&wreck(1, "copsuv")), 0.016, plain);
        assert_ne!(line.message().map(|m| m.serial), first, "the message group plays it again");
        assert_eq!(line.message().map(|m| m.text.as_str()), Some("copsuv 500"));
    }

    #[test]
    fn the_first_look_and_a_lower_count_are_only_baselines() {
        let mut line = TakedownLine::default();
        line.observe_with(Some(&wreck(3, "copmidsize")), 0.0, plain);
        assert!(line.message().is_none(), "a world that opens into a wreck does not tell it");
        line.observe_with(None, 0.016, plain);
        assert_eq!(line.count, Some(0), "a new chase starts the count again");
        line.observe_with(Some(&wreck(1, "traffictaxi")), 0.016, plain);
        assert!(line.message().is_none(), "a car with no line tells nothing");
        assert_eq!(line.count, Some(1));
    }
}
