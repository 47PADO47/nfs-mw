//! The bust after the busted meter fills: hold the car for the arrest shot, play the arrest scene as a NIS, then show
//! the infractions screen in free roam. The world plays the scene: it takes the request ([`BustFlow::take_arrest_request`])
//! and answers with [`BustFlow::arrest_scene_over`] when the scene ends or cannot play. A scene that cannot play ends the
//! wait at once, so the ticket follows the same 3 s hold as before. Nothing here reads a clock or a screen.
//! Spec: `docs/specs/busted-sequence.md` sections 2, 4, 6 and 7.

use blackbox_pursuit::{Fines, Infraction, InfractionLog};

use super::arrest;

/// The delay before the arrest shot, when the arrest scene plays (the original's NIS request). A scene that cannot play
/// is answered at this point too, so the ticket follows this hold.
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
    /// The sequence is over: a race's arrest scene has ended, or the player has paid in free roam.
    Finished,
    /// The arrest shot is reached: the world plays this scene as the NIS and answers with [`BustFlow::arrest_scene_over`].
    PlayArrest(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Stage {
    /// No bust this chase.
    #[default]
    Idle,
    /// The arrest shot's hold, with the scene the chooser picked.
    Arrest {
        secs: f32,
        scene: &'static str,
    },
    /// The arrest scene is asked for and plays; the sequence waits for its end.
    ArrestScene,
    /// The infractions screen is up and waits for the payment.
    Infractions,
    Finished,
}

/// The bust sequence of one chase. [`BustFlow::reset`] at each new chase; a bust is started once per chase.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BustFlow {
    stage: Stage,
    race: bool,
    /// The arrest scene the chooser picked for the last bust (`arrest_scene`).
    arrest: Option<&'static str>,
    report: BustReport,
    /// The arrest scene waiting for the world (`take_arrest_request`): set at the arrest shot.
    arrest_request: Option<&'static str>,
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
        let scene = arrest::arrest_scene(class, arrest::variant_from_roll(roll));
        self.stage = Stage::Arrest { secs: 0.0, scene };
        self.race = race;
        self.arrest = Some(scene);
        self.report = report;
        true
    }

    /// A new chase starts: no bust is on. The arrest scene of the last bust stays until the next one.
    pub fn reset(&mut self) {
        self.stage = Stage::Idle;
        self.report = BustReport::default();
        self.arrest_request = None;
        self.request = None;
    }

    /// The arrest scene to play as the NIS, once: the world takes it when the arrest shot has been reached.
    pub fn take_arrest_request(&mut self) -> Option<&'static str> {
        self.arrest_request.take()
    }

    /// The arrest scene is over: it ended, or the world could not play it. The hold is already spent, so the sequence
    /// goes on at once: the infractions screen in free roam, the end of the bust in a race. Does nothing unless the
    /// scene is the one playing.
    pub fn arrest_scene_over(&mut self) -> Option<Step> {
        if self.stage != Stage::ArrestScene {
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
            Stage::Arrest { secs, scene } => self.run_arrest(secs + dt, scene),
            Stage::Idle | Stage::ArrestScene | Stage::Infractions | Stage::Finished => None,
        }
    }

    /// The arrest shot: the scene is asked for and the sequence waits for its end ([`BustFlow::arrest_scene_over`]).
    fn run_arrest(&mut self, secs: f32, scene: &'static str) -> Option<Step> {
        if secs < SHOT_SECS {
            self.stage = Stage::Arrest { secs, scene };
            return None;
        }
        self.stage = Stage::ArrestScene;
        self.arrest_request = Some(scene);
        Some(Step::PlayArrest(scene))
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

    /// Whether the in-game overlay shows. This sequence never shows it: its show path is not located (`busted-sequence.md`
    /// section 6.1).
    pub fn overlay_shown(&self) -> bool {
        false
    }

    /// Whether this bust takes the place of the end-of-chase card: a free-roam bust does (its infractions screen does
    /// the job), a race keeps the card.
    pub fn replaces_end_card(&self) -> bool {
        !self.race && self.stage != Stage::Idle
    }

    /// The arrest scene chosen for the last bust.
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

    /// Ticks a free-roam bust through the arrest shot, then answers the scene as over (it cannot play, or it has ended),
    /// which brings up the ticket.
    fn to_the_screen(flow: &mut BustFlow) {
        assert!(matches!(flow.tick(SHOT_SECS + 0.1), Some(Step::PlayArrest(_))));
        assert!(matches!(flow.arrest_scene_over(), Some(Step::Infractions(_))));
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
    fn a_free_roam_bust_holds_then_asks_for_the_arrest_scene_before_the_ticket() {
        let mut flow = free_roam_bust();
        assert_eq!(flow.tick(SHOT_SECS / 2.0), None, "the arrest shot runs first");
        assert!(!flow.overlay_shown());
        let step = flow.tick(SHOT_SECS / 2.0 + 0.01);
        assert!(matches!(step, Some(Step::PlayArrest(_))), "{step:?}");
        assert!(flow.take_request().is_none(), "no ticket until the scene is over");
        assert!(!flow.overlay_shown());
        assert!(flow.active(), "the bust keeps controls held through the scene");
    }

    #[test]
    fn the_arrest_scene_is_the_one_the_chooser_picked() {
        let mut flow = BustFlow::default();
        assert!(flow.start(false, 2, 0.99, BustReport::default()));
        assert_eq!(flow.tick(SHOT_SECS + 0.1), Some(Step::PlayArrest("ArrestF07b")));
        assert_eq!(flow.take_arrest_request(), Some("ArrestF07b"));
        assert_eq!(flow.take_arrest_request(), None, "handed over once");
    }

    #[test]
    fn with_the_arrest_scene_playing_the_sequence_waits_for_its_end() {
        let mut flow = free_roam_bust();
        assert!(matches!(flow.tick(SHOT_SECS + 0.1), Some(Step::PlayArrest(_))));
        assert_eq!(flow.tick(10.0), None, "the scene plays on; the hold is not the end");
        assert!(flow.active() && flow.take_request().is_none());
        assert_eq!(flow.pay(Payment::Cash, 0), Err(PayError::NotDue), "no payment during the scene");
        assert!(matches!(flow.arrest_scene_over(), Some(Step::Infractions(ref r)) if r.total == 500));
        assert_eq!(flow.take_request().map(|r| r.total), Some(500), "the ticket follows the scene");
    }

    #[test]
    fn with_no_arrest_scene_to_play_the_three_second_hold_still_ends_the_wait() {
        let mut flow = free_roam_bust();
        assert_eq!(flow.tick(SHOT_SECS - 0.1), None);
        assert!(matches!(flow.tick(0.2), Some(Step::PlayArrest(_))));
        assert!(matches!(flow.arrest_scene_over(), Some(Step::Infractions(_))), "the world could not play it");
        assert_eq!(flow.take_request().map(|r| r.total), Some(500));
    }

    #[test]
    fn a_race_with_the_scene_ending_or_refused_finishes_the_bust() {
        let mut flow = BustFlow::default();
        assert!(flow.start(true, 0, 0.0, BustReport::default()));
        assert!(!flow.replaces_end_card(), "the race keeps its end card");
        assert_eq!(flow.tick(SHOT_SECS - 0.1), None);
        assert_eq!(flow.tick(0.2), Some(Step::PlayArrest("ArrestM01")));
        assert_eq!(flow.tick(5.0), None, "the scene plays on");
        assert_eq!(flow.arrest_scene_over(), Some(Step::Finished));
        assert!(!flow.active() && !flow.overlay_shown());
        assert_eq!(flow.tick(5.0), None);
    }

    #[test]
    fn a_scene_over_before_the_shot_or_twice_changes_nothing() {
        let mut flow = free_roam_bust();
        assert_eq!(flow.arrest_scene_over(), None, "no scene is playing during the hold");
        assert!(flow.active() && flow.take_request().is_none());
        assert!(matches!(flow.tick(SHOT_SECS + 0.1), Some(Step::PlayArrest(_))));
        assert!(matches!(flow.arrest_scene_over(), Some(Step::Infractions(_))));
        assert_eq!(flow.arrest_scene_over(), None, "the scene is over once");
    }

    #[test]
    fn an_overshooting_step_asks_for_the_scene_once() {
        let mut flow = free_roam_bust();
        assert!(matches!(flow.tick(SHOT_SECS + 2.0), Some(Step::PlayArrest(_))));
        assert_eq!(flow.tick(SHOT_SECS), None);
        assert!(matches!(flow.arrest_scene_over(), Some(Step::Infractions(_))));
    }

    #[test]
    fn the_ticket_is_taken_once_when_the_infractions_step_begins() {
        let mut flow = free_roam_bust();
        assert!(flow.take_request().is_none(), "no ticket during the arrest");
        flow.tick(SHOT_SECS + 0.1);
        assert!(flow.take_request().is_none(), "the scene is still to be answered");
        flow.arrest_scene_over();
        flow.tick(SHOT_SECS + 0.1);
        assert_eq!(flow.take_request().map(|r| r.total), Some(500));
        assert!(flow.take_request().is_none());
    }

    #[test]
    fn the_steps_fire_once() {
        let mut flow = free_roam_bust();
        to_the_screen(&mut flow);
        assert_eq!(flow.tick(10.0), None, "the infractions screen waits for the payment, nothing more ticks");
    }

    #[test]
    fn the_free_roam_bust_replaces_the_end_card_from_the_start() {
        let flow = free_roam_bust();
        assert!(flow.replaces_end_card());
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
