//! The escape cool-down as pure rules (`docs/specs/ai-pursuit.md` sections 4.3 and 6). Once no cop has seen the player
//! for the grace window, the cool-down meter fills towards getting away. A gate of `mode_gate` seconds keeps the chase
//! and the cool-down display from flickering: the meter comes up only after the gate, and a sighting drains the shown
//! meter by `decay` each update until the gate lets the chase display back. A meter of 1 while shown is EVADED, which
//! is terminal.
//!
//! Nothing here reads a world or a clock: the caller says whether a cop sees the player, whether the player is in a
//! hiding spot, the heat's timeout and hiding multiplier, and the frame time. The tuning is passed in; [`TUNING`] holds
//! the install's values.

/// The cool-down's tuning. The caller passes it in; [`TUNING`] holds the values the spec gives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CooldownTuning {
    /// Seconds without a sighting before the chase counts as lost (`mIsPerpInSight`, 7.0).
    pub grace: f32,
    /// Seconds between the chase display and the cool-down display, each way (`PursuitMeterModeTimer`, 2.5).
    pub mode_gate: f32,
    /// The factor the shown meter keeps each update while a cop sees the player (0.93).
    pub decay: f32,
    /// The least the shown meter falls to while it is shown (0.05).
    pub floor: f32,
}

/// The install's cool-down values (`docs/specs/ai-pursuit.md` section 6 and the constants table).
pub const TUNING: CooldownTuning = CooldownTuning { grace: 7.0, mode_gate: 2.5, decay: 0.93, floor: 0.05 };

/// The sight timer at the start of a chase, seconds (`time_since_any_saw` starts at -5, section 4.3).
pub const SIGHT_AT_START: f32 = -5.0;

/// The sight timer after a sighting, seconds: the value a cop's visibility test leaves (section 4.1, -0.25).
const SIGHTED: f32 = -0.25;

/// What the caller measured this frame for the cool-down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CooldownInput {
    /// A cop can see the player.
    pub seen: bool,
    /// The player is in a hiding spot (a HIDDEN zone at its height).
    pub hidden: bool,
    /// Unseen seconds needed to evade at the heat level (`evade::timeout`).
    pub required: f32,
    /// Progress multiplier inside a hiding spot at the heat level (`evade::hidden_multiplier`).
    pub hidden_multiplier: f32,
}

/// Where the chase is in the escape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The chase runs; the cops see the player or the grace has not passed.
    InitialChase,
    /// The cops have lost the player and the cool-down meter is shown.
    CoolDown,
    /// The meter reached 1 while shown: the player got away. Terminal.
    Evaded,
}

/// The escape state of one chase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cooldown {
    /// Seconds since a cop last saw the player (`time_since_any_saw`): negative just after a sighting.
    since_seen: f32,
    /// Hiding time counted at the multiplier, unseen past the grace (`hidden_zone_time`).
    hidden_time: f32,
    /// The cool-down meter, 0 to 1 (`evade_level`).
    level: f32,
    /// Unseen seconds to evade, frozen while the cool-down is shown (`CoolDownTimeRequired`).
    required: f32,
    /// Seconds since the last change of display (`PursuitMeterModeTimer`).
    mode_timer: f32,
    /// The cool-down is shown (`CoolDownMeterDisplayed`).
    displayed: bool,
    status: Status,
}

impl Default for Cooldown {
    fn default() -> Self {
        Self {
            since_seen: SIGHT_AT_START,
            hidden_time: 0.0,
            level: 0.0,
            required: 0.0,
            mode_timer: 0.0,
            displayed: false,
            status: Status::InitialChase,
        }
    }
}

impl Cooldown {
    /// Advances by `dt` seconds with this frame's input and returns the status after the step. Evaded is terminal, and
    /// a bad `dt` changes nothing.
    pub fn update(&mut self, dt: f32, input: &CooldownInput, tuning: &CooldownTuning) -> Status {
        if self.status == Status::Evaded || !(dt > 0.0 && dt.is_finite()) {
            return self.status;
        }
        if !self.displayed {
            self.required = input.required;
        }
        self.mode_timer += dt;
        self.since_seen = match input.seen {
            true => SIGHTED,
            false => self.since_seen + dt,
        };
        let in_sight = self.since_seen < tuning.grace;
        self.hidden_time = match (in_sight, input.hidden) {
            (true, _) => 0.0,
            (false, true) => self.hidden_time + dt * input.hidden_multiplier,
            (false, false) => self.hidden_time,
        };
        if in_sight {
            self.sighted(tuning);
            return self.status;
        }
        self.lost(tuning);
        self.status
    }

    /// A cop saw the player within the grace. Without the display the meter stays at 0; with it, the meter drains by
    /// the decay, and the display goes back to the chase once the gate has passed.
    fn sighted(&mut self, tuning: &CooldownTuning) {
        if !self.displayed {
            self.level = 0.0;
            return;
        }
        self.level = (self.level * tuning.decay).max(tuning.floor);
        if self.mode_timer <= tuning.mode_gate {
            return;
        }
        self.displayed = false;
        self.status = Status::InitialChase;
        self.mode_timer = 0.0;
    }

    /// The cops have lost the player past the grace. Without the display the meter stays at 0 until the gate has
    /// passed; with it, the meter fills and reaching 1 is EVADED.
    fn lost(&mut self, tuning: &CooldownTuning) {
        if !self.displayed {
            self.level = 0.0;
            if self.mode_timer <= tuning.mode_gate {
                return;
            }
            self.displayed = true;
            self.status = Status::CoolDown;
            self.mode_timer = 0.0;
            return;
        }
        self.level = self.share().max(tuning.floor);
        if self.level < 1.0 {
            return;
        }
        self.level = 1.0;
        self.status = Status::Evaded;
    }

    /// The unseen progress over the required time: the meter the fill follows.
    fn share(&self) -> f32 {
        self.progress() / self.required.max(f32::EPSILON)
    }

    /// The seconds of progress so far, `time_since_any_saw` plus the hiding time. It starts negative and is what the
    /// audio feed reads against the timeout.
    pub fn progress(&self) -> f32 {
        self.since_seen + self.hidden_time
    }

    /// Raw time since a sighting, excluding the extra progress earned in hiding.
    pub fn since_seen(&self) -> f32 {
        self.since_seen
    }

    /// The cool-down meter, 0 to 1 (`evade_level`).
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Where the chase is in the escape.
    pub fn status(&self) -> Status {
        self.status
    }

    /// Whether the cool-down meter is on the screen.
    pub fn is_shown(&self) -> bool {
        self.displayed
    }

    /// The unseen seconds to evade at the heat level, as frozen while the cool-down shows.
    pub fn required(&self) -> f32 {
        self.required
    }

    /// The fill of the pursuit board's cool-down bar, 0 to 1: `1 - remaining / (required - grace)`. The bar spans only
    /// the cool-down's own length, so it opens empty even though the meter ([`level`](Self::level)) already holds the
    /// grace's share of the required time. Empty until the cool-down shows, full once evaded.
    pub fn bar(&self, tuning: &CooldownTuning) -> f32 {
        if self.status == Status::Evaded {
            return 1.0;
        }
        if !self.displayed {
            return 0.0;
        }
        let length = self.required - tuning.grace;
        if length <= f32::EPSILON {
            return 1.0;
        }
        (1.0 - self.remaining(tuning) / length).clamp(0.0, 1.0)
    }

    /// Seconds still to go to evade, for the status line: `required - progress`, clamped to the cool-down's length
    /// (`required - grace`, spec section 6).
    pub fn remaining(&self, tuning: &CooldownTuning) -> f32 {
        (self.required - self.progress()).clamp(0.0, (self.required - tuning.grace).max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 0.05;

    fn unseen(required: f32, hidden: bool) -> CooldownInput {
        CooldownInput { seen: false, hidden, required, hidden_multiplier: 4.0 }
    }

    fn seen(required: f32) -> CooldownInput {
        CooldownInput { seen: true, ..unseen(required, false) }
    }

    /// Steps `secs` seconds with `input` and returns the status at the end.
    fn step(cooldown: &mut Cooldown, secs: f32, input: CooldownInput) -> Status {
        let mut status = cooldown.status();
        for _ in 0..(secs / DT).round() as usize {
            status = cooldown.update(DT, &input, &TUNING);
        }
        status
    }

    #[test]
    fn a_chase_starts_with_the_chase_shown_and_the_meter_empty() {
        let cooldown = Cooldown::default();
        assert_eq!(cooldown.status(), Status::InitialChase);
        assert!(!cooldown.is_shown());
        assert_eq!(cooldown.level(), 0.0);
    }

    #[test]
    fn the_cool_down_shows_after_the_grace_and_the_gate() {
        let mut cooldown = Cooldown::default();
        // The sight timer starts at -5, so the grace (7 s) ends 12 s into the chase.
        step(&mut cooldown, 11.5, unseen(20.0, false));
        assert!(!cooldown.is_shown(), "still inside the grace");
        let status = step(&mut cooldown, 1.0, unseen(20.0, false));
        assert_eq!(status, Status::CoolDown);
        assert!(cooldown.is_shown());
        assert!((0.3..=0.45).contains(&cooldown.level()), "about 7.5 of 20 s: {}", cooldown.level());
    }

    #[test]
    fn a_sighting_inside_the_grace_resets_it() {
        let mut cooldown = Cooldown::default();
        step(&mut cooldown, 11.0, unseen(20.0, false));
        step(&mut cooldown, DT, seen(20.0));
        // Two more seconds unseen would pass the grace (6 + 2 > 7) if the sighting had not reset it.
        let status = step(&mut cooldown, 2.0, unseen(20.0, false));
        assert_eq!(status, Status::InitialChase);
        assert!(!cooldown.is_shown(), "the sighting started the grace again");
        assert_eq!(cooldown.level(), 0.0);
    }

    #[test]
    fn a_sighting_drains_the_shown_meter_by_the_decay() {
        let mut cooldown = Cooldown::default();
        step(&mut cooldown, 12.5, unseen(20.0, false));
        let before = cooldown.level();
        step(&mut cooldown, DT, seen(20.0));
        assert!((cooldown.level() - before * TUNING.decay).abs() < 1e-4, "{} -> {}", before, cooldown.level());
    }

    #[test]
    fn the_shown_meter_holds_its_floor_inside_the_gate() {
        let mut cooldown = Cooldown::default();
        step(&mut cooldown, 12.5, unseen(20.0, false));
        // 1.5 s of sighting is under the 2.5 s gate: the display stays and the meter falls to its floor.
        step(&mut cooldown, 1.5, seen(20.0));
        assert!(cooldown.is_shown(), "the gate is not past yet");
        assert_eq!(cooldown.level(), TUNING.floor);
    }

    #[test]
    fn the_chase_comes_back_once_the_gate_has_passed() {
        let mut cooldown = Cooldown::default();
        step(&mut cooldown, 12.5, unseen(20.0, false));
        let status = step(&mut cooldown, 2.6, seen(20.0));
        assert_eq!(status, Status::InitialChase);
        assert!(!cooldown.is_shown());
        assert_eq!(cooldown.level(), 0.0);
    }

    #[test]
    fn staying_hidden_evades_sooner_than_staying_unseen() {
        let mut hidden = Cooldown::default();
        // Hidden from 12 s on at multiplier 4: progress 5 per second after the grace, so 20 is reached near 14.6 s.
        step(&mut hidden, 14.0, unseen(20.0, true));
        assert_eq!(hidden.status(), Status::CoolDown, "not yet");
        assert_eq!(step(&mut hidden, 1.0, unseen(20.0, true)), Status::Evaded);

        let mut open = Cooldown::default();
        step(&mut open, 20.0, unseen(20.0, false));
        assert_eq!(open.status(), Status::CoolDown, "unseen and not hiding needs until 25 s");
        assert_eq!(step(&mut open, 6.0, unseen(20.0, false)), Status::Evaded);
    }

    #[test]
    fn evaded_is_terminal() {
        let mut cooldown = Cooldown::default();
        step(&mut cooldown, 20.0, unseen(20.0, true));
        assert_eq!(cooldown.status(), Status::Evaded);
        assert_eq!(cooldown.level(), 1.0);
        assert_eq!(step(&mut cooldown, 5.0, seen(20.0)), Status::Evaded, "a sighting does not undo it");
    }

    #[test]
    fn the_required_time_is_frozen_while_the_cool_down_shows() {
        let mut cooldown = Cooldown::default();
        step(&mut cooldown, 12.5, unseen(20.0, false));
        step(&mut cooldown, DT, seen(45.0));
        assert_eq!(cooldown.required(), 20.0, "a new heat does not move a shown meter");
    }

    #[test]
    fn the_remaining_time_is_clamped_to_the_cool_down_length() {
        let mut cooldown = Cooldown::default();
        step(&mut cooldown, DT, unseen(20.0, false));
        // 20 + 5 seconds is more than the 13 s cool-down (20 - 7), so the remaining time is clamped to 13.
        assert_eq!(cooldown.remaining(&TUNING), 13.0);
        step(&mut cooldown, 12.5, unseen(20.0, false));
        assert!((cooldown.remaining(&TUNING) - 12.5).abs() < 0.2, "{}", cooldown.remaining(&TUNING));
    }

    #[test]
    fn the_board_bar_opens_empty_while_the_meter_holds_the_grace() {
        let mut cooldown = Cooldown::default();
        assert_eq!(cooldown.bar(&TUNING), 0.0, "no cool-down yet");
        step(&mut cooldown, 11.9, unseen(20.0, false));
        assert_eq!(cooldown.bar(&TUNING), 0.0, "still inside the grace");
        step(&mut cooldown, 0.6, unseen(20.0, false));
        assert!(cooldown.is_shown());
        assert!(cooldown.level() > 0.3, "the meter already holds the grace: {}", cooldown.level());
        assert!(cooldown.bar(&TUNING) < 0.06, "the bar starts empty: {}", cooldown.bar(&TUNING));
        // Half of the 13 s cool-down later the bar is about half full.
        step(&mut cooldown, 6.5, unseen(20.0, false));
        assert!((cooldown.bar(&TUNING) - 0.5).abs() < 0.08, "{}", cooldown.bar(&TUNING));
    }

    #[test]
    fn the_board_bar_ends_full_when_evaded_and_falls_back_to_empty_when_the_chase_returns() {
        let mut evaded = Cooldown::default();
        step(&mut evaded, 20.0, unseen(20.0, true));
        assert_eq!(evaded.bar(&TUNING), 1.0);
        let mut chase = Cooldown::default();
        step(&mut chase, 14.0, unseen(20.0, false));
        step(&mut chase, 2.6, seen(20.0));
        assert_eq!(chase.bar(&TUNING), 0.0);
    }

    #[test]
    fn a_bad_step_changes_nothing() {
        let mut cooldown = Cooldown::default();
        step(&mut cooldown, 12.5, unseen(20.0, false));
        let before = cooldown;
        cooldown.update(f32::NAN, &unseen(20.0, false), &TUNING);
        cooldown.update(-1.0, &unseen(20.0, false), &TUNING);
        assert_eq!(cooldown, before);
    }
}
