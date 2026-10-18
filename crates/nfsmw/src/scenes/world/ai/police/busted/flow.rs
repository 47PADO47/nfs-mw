//! The fallback after the busted meter fills: hold the car, then show the infractions screen in free roam.
//! The original's NIS player is unavailable; the no-NIS path does not invent an extra overlay delay. The world starts it with
//! the bust and the frame's time; the front end carries out the steps it asks for and reports the payment back.
//! Nothing here reads a clock or a screen. Spec: `docs/specs/busted-sequence.md` sections 2, 6 and 7.

use blackbox_pursuit::{Fines, Infraction, InfractionLog};

use super::arrest;

/// The original's delay before requesting the arrest NIS. Without NIS playback the ticket follows this hold.
pub const SHOT_SECS: f32 = 3.0;

/// One infraction of the chase and its fine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
    pub infraction: Infraction,
    pub fine: u32,
}

/// What the infractions screen shows: each infraction of the chase with its fine, and the total due.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BustReport {
    pub lines: Vec<Line>,
    pub total: u32,
}

impl BustReport {
    /// The report of a chase's log: each recorded infraction in HUD order with its fine, and the total of the fines.
    pub fn from_log(log: &InfractionLog, fines: &Fines) -> Self {
        let lines: Vec<Line> = log.iter().map(|infraction| Line { infraction, fine: fines.of(infraction) }).collect();
        let total = lines.iter().fold(0u32, |sum, line| sum.saturating_add(line.fine));
        Self { lines, total }
    }
}

/// How the player pays on the infractions screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payment {
    Cash,
    /// The Get Out of Jail Free marker. Needs one in stock; its effect on the car's strikes is not in the port yet.
    Marker,
}

/// Why a payment was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayError {
    /// The infractions screen is not up (no bust is waiting for payment).
    NotDue,
    /// The player has no marker to pay with.
    NoMarker,
}

/// What the sequence asks the game to do when a step ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Show the infractions screen with this report; the front end answers with [`BustFlow::pay`].
    Infractions(BustReport),
    /// The sequence is over: a race's arrest shot has run, or the player has paid in free roam.
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Stage {
    /// No bust this chase.
    #[default]
    Idle,
    Arrest {
        secs: f32,
    },
    /// The infractions screen is up and waits for the payment.
    Infractions,
    Finished,
}

/// The bust sequence of one chase. [`BustFlow::reset`] at each new chase; a bust is started once per chase.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BustFlow {
    stage: Stage,
    race: bool,
    arrest: Option<&'static str>,
    report: BustReport,
    /// The ticket waiting for the front end (`take_request`): set when the infractions step begins.
    request: Option<BustReport>,
}

impl BustFlow {
    pub fn active(&self) -> bool {
        !matches!(self.stage, Stage::Idle | Stage::Finished)
    }

    /// Starts the sequence for a bust. `class` and `roll` pick the arrest scene (`arrest.rs`); `report` is the chase's
    /// infractions. Returns `false` (and changes nothing) when a bust is already on in this chase.
    pub fn start(&mut self, race: bool, class: u8, roll: f32, report: BustReport) -> bool {
        if self.stage != Stage::Idle {
            return false;
        }
        self.stage = Stage::Arrest { secs: 0.0 };
        self.race = race;
        self.arrest = Some(arrest::arrest_scene(class, arrest::variant_from_roll(roll)));
        self.report = report;
        true
    }

    /// A new chase starts: no bust is on. The arrest scene of the last bust stays until the next one.
    pub fn reset(&mut self) {
        self.stage = Stage::Idle;
        self.report = BustReport::default();
        self.request = None;
    }

    /// The ticket to show, once: the front end takes it when the infractions step has begun.
    pub fn take_request(&mut self) -> Option<BustReport> {
        self.request.take()
    }

    /// Advances the sequence by `dt` seconds. Returns the step that ended this frame, if one did.
    pub fn tick(&mut self, dt: f32) -> Option<Step> {
        if !dt.is_finite() || dt <= 0.0 {
            return None;
        }
        match self.stage {
            Stage::Arrest { secs } => self.run_arrest(secs + dt),
            Stage::Idle | Stage::Infractions | Stage::Finished => None,
        }
    }

    fn run_arrest(&mut self, secs: f32) -> Option<Step> {
        if secs < SHOT_SECS {
            self.stage = Stage::Arrest { secs };
            return None;
        }
        if self.race {
            self.stage = Stage::Finished;
            return Some(Step::Finished);
        }
        self.stage = Stage::Infractions;
        self.request = Some(self.report.clone());
        Some(Step::Infractions(self.report.clone()))
    }

    /// The player pays on the infractions screen. `markers` is the markers in stock (the port has no career state, so
    /// the caller passes 0 until it does). Cash always works; a marker needs one in stock. The sequence ends on success.
    pub fn pay(&mut self, payment: Payment, markers: u32) -> Result<(), PayError> {
        if self.stage != Stage::Infractions {
            return Err(PayError::NotDue);
        }
        if payment == Payment::Marker && markers == 0 {
            return Err(PayError::NoMarker);
        }
        self.stage = Stage::Finished;
        Ok(())
    }

    /// The fallback has no arrest NIS completion event to trigger an overlay.
    pub fn overlay_shown(&self) -> bool {
        false
    }

    /// Whether this bust takes the place of the end-of-chase card: a free-roam bust does (its infractions screen does
    /// the job), a race keeps the card.
    pub fn replaces_end_card(&self) -> bool {
        !self.race && self.stage != Stage::Idle
    }

    /// The arrest scene chosen for the last bust. Nothing plays it yet (the NIS player is not in the port).
    pub fn arrest_scene(&self) -> Option<&'static str> {
        self.arrest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FINES: Fines = Fines::new([150, 350, 1000, 350, 300, 100, 300, 75]);

    fn log_of(infractions: &[Infraction]) -> InfractionLog {
        let mut log = InfractionLog::default();
        for infraction in infractions {
            log.record(*infraction, &FINES);
        }
        log
    }

    /// Ticks a free-roam bust through the NIS request delay to the fallback ticket.
    fn to_the_screen(flow: &mut BustFlow) {
        assert!(matches!(flow.tick(SHOT_SECS + 0.1), Some(Step::Infractions(_))));
    }

    fn free_roam_bust() -> BustFlow {
        let mut flow = BustFlow::default();
        let report = BustReport::from_log(&log_of(&[Infraction::Speeding, Infraction::Racing]), &FINES);
        assert!(flow.start(false, 0, 0.0, report));
        flow
    }

    #[test]
    fn the_report_lists_each_infraction_with_its_fine_and_the_total() {
        let log = log_of(&[Infraction::OffRoad, Infraction::Speeding]);
        let report = BustReport::from_log(&log, &FINES);
        let lines: Vec<(Infraction, u32)> = report.lines.iter().map(|l| (l.infraction, l.fine)).collect();
        assert_eq!(lines, [(Infraction::Speeding, 150), (Infraction::OffRoad, 75)], "HUD order, not recording order");
        assert_eq!(report.total, 225);
        assert_eq!(report.total, log.total_fines(), "the same total as the log keeps");
    }

    #[test]
    fn an_empty_chase_owes_nothing() {
        let report = BustReport::from_log(&InfractionLog::default(), &FINES);
        assert!(report.lines.is_empty());
        assert_eq!(report.total, 0);
    }

    #[test]
    fn the_total_saturates_instead_of_overflowing() {
        let huge = Fines::new([u32::MAX; 8]);
        let mut log = InfractionLog::default();
        log.record(Infraction::Speeding, &huge);
        log.record(Infraction::Racing, &huge);
        assert_eq!(BustReport::from_log(&log, &huge).total, u32::MAX);
    }

    #[test]
    fn nothing_happens_before_a_bust_starts() {
        let mut flow = BustFlow::default();
        assert_eq!(flow.tick(10.0), None);
        assert!(!flow.overlay_shown());
        assert!(!flow.replaces_end_card());
    }

    #[test]
    fn invalid_time_cannot_skip_the_hold_or_corrupt_the_fallback_clock() {
        let mut flow = free_roam_bust();
        let before = flow.clone();
        for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(flow.tick(dt), None);
            assert_eq!(flow, before);
        }
    }

    #[test]
    fn a_free_roam_bust_holds_then_requests_the_ticket_without_a_fake_nis_delay() {
        let mut flow = free_roam_bust();
        assert_eq!(flow.tick(SHOT_SECS / 2.0), None, "the arrest shot runs first");
        assert!(!flow.overlay_shown());
        let step = flow.tick(SHOT_SECS / 2.0 + 0.01);
        assert!(matches!(step, Some(Step::Infractions(ref r)) if r.total == 500), "{step:?}");
        assert!(!flow.overlay_shown());
        assert!(flow.active(), "the ticket keeps controls held until payment");
    }

    #[test]
    fn an_overshooting_step_requests_only_one_ticket() {
        let mut flow = free_roam_bust();
        assert!(matches!(flow.tick(SHOT_SECS + 2.0), Some(Step::Infractions(_))));
        assert_eq!(flow.tick(SHOT_SECS), None);
    }

    #[test]
    fn the_ticket_is_taken_once_when_the_infractions_step_begins() {
        let mut flow = free_roam_bust();
        assert!(flow.take_request().is_none(), "no ticket during the arrest");
        flow.tick(SHOT_SECS + 0.1);
        flow.tick(SHOT_SECS + 0.1);
        assert_eq!(flow.take_request().map(|r| r.total), Some(500));
        assert!(flow.take_request().is_none());
    }

    #[test]
    fn the_steps_fire_once() {
        let mut flow = free_roam_bust();
        flow.tick(SHOT_SECS + 0.1);
        flow.tick(SHOT_SECS + 0.1);
        assert_eq!(flow.tick(10.0), None, "the infractions screen waits for the payment, nothing more ticks");
    }

    #[test]
    fn the_free_roam_bust_replaces_the_end_card_from_the_start() {
        let flow = free_roam_bust();
        assert!(flow.replaces_end_card());
    }

    #[test]
    fn a_race_bust_ends_after_the_arrest_shot_and_keeps_the_card() {
        let mut flow = BustFlow::default();
        assert!(flow.start(true, 0, 0.0, BustReport::default()));
        assert!(!flow.replaces_end_card(), "the race keeps its end card");
        assert_eq!(flow.tick(SHOT_SECS - 0.1), None);
        assert_eq!(flow.tick(0.2), Some(Step::Finished));
        assert!(!flow.overlay_shown(), "no overlay in a race");
        assert_eq!(flow.tick(5.0), None);
    }

    #[test]
    fn a_second_start_in_the_same_chase_changes_nothing() {
        let mut flow = free_roam_bust();
        let before = flow.clone();
        assert!(!flow.start(true, 2, 0.9, BustReport::default()));
        assert_eq!(flow, before);
    }

    #[test]
    fn reset_readies_the_sequence_for_the_next_chase() {
        let mut flow = free_roam_bust();
        to_the_screen(&mut flow);
        flow.pay(Payment::Cash, 0).unwrap();
        flow.reset();
        assert!(!flow.replaces_end_card());
        assert!(flow.report.lines.is_empty());
        assert!(flow.start(false, 2, 0.0, BustReport::default()), "a new bust starts after the reset");
    }

    #[test]
    fn the_arrest_scene_is_chosen_by_class_and_roll() {
        let mut flow = BustFlow::default();
        flow.start(false, 0, 0.0, BustReport::default());
        assert_eq!(flow.arrest_scene(), Some("ArrestM01"));
        flow.reset();
        flow.start(false, 2, 0.99, BustReport::default());
        assert_eq!(flow.arrest_scene(), Some("ArrestF07b"));
    }

    #[test]
    fn cash_pays_once_the_screen_is_up_and_ends_the_sequence() {
        let mut flow = free_roam_bust();
        assert_eq!(flow.pay(Payment::Cash, 0), Err(PayError::NotDue), "no payment during the arrest");
        to_the_screen(&mut flow);
        assert_eq!(flow.pay(Payment::Cash, 0), Ok(()));
        assert_eq!(flow.pay(Payment::Cash, 0), Err(PayError::NotDue), "paid once");
    }

    #[test]
    fn a_marker_needs_one_in_stock() {
        let mut flow = free_roam_bust();
        to_the_screen(&mut flow);
        assert_eq!(flow.pay(Payment::Marker, 0), Err(PayError::NoMarker));
        assert_eq!(flow.pay(Payment::Cash, 0), Ok(()), "a refused marker leaves the screen up for cash");
    }

    #[test]
    fn a_marker_pays_when_one_is_in_stock() {
        let mut flow = free_roam_bust();
        to_the_screen(&mut flow);
        assert_eq!(flow.pay(Payment::Marker, 1), Ok(()));
    }
}
