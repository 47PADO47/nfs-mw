//! The car side of the siren: the per-car state machine. Spec: `docs/specs/cop-siren-audio.md` section 2.

use super::State;

/// The siren times of the current heat level, seconds (six fields of the level's data row).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Opening exchange: the shortest gap between two yelps.
    pub init_min_period: f32,
    /// Opening exchange: the random extra on that gap.
    pub init_variation: f32,
    /// How long the wail runs before the yelp interlude.
    pub wail_period: f32,
    /// How long the yelp interlude lasts.
    pub max_yelp_time: f32,
    /// A pressing formation: the quiet between two screams.
    pub scream_period: f32,
    /// A pressing formation: how long a scream lasts.
    pub max_scream_time: f32,
}

impl Default for Tuning {
    /// The values every heat row of the shipped data has ([verified] on the install; see the spec).
    fn default() -> Self {
        Self {
            init_min_period: 0.5,
            init_variation: 1.5,
            wail_period: 11.0,
            max_yelp_time: 6.0,
            scream_period: 60.0,
            max_scream_time: 3.0,
        }
    }
}

/// Where the chase is, as far as the siren cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    /// The first seconds of the player's only chase, while the dispatch's opening exchange runs: the sirens do not
    /// run; each car yelps briefly now and then.
    Opening,
    /// Contact with the player is lost: the sirens go quiet and the cars' chase clocks restart.
    Lost,
    /// Anything else.
    #[default]
    Running,
}

/// What the machine reads each tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Input {
    /// The car is chasing the player (and the chase has its tuning).
    pub in_pursuit: bool,
    pub phase: Phase,
    /// The chase is intense: the heat is at the mark, or the chase has run long.
    pub high_intensity: bool,
    /// The cars press the player now (a formation collapsing, a finisher running, or a closing line being said).
    pub pressing: bool,
    /// The car is part of the formation.
    pub in_formation: bool,
    /// The car is one of a roadblock.
    pub in_roadblock: bool,
    /// The car's light bar is lit.
    pub lights_on: bool,
    /// The car is wrecked.
    pub destroyed: bool,
}

/// The gap between the opening yelps, shared by every car (the original keeps one value for all of them, so the
/// cars yelp in step). It is drawn when first needed and again whenever a car finishes a yelp.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OpeningGap(Option<f32>);

impl OpeningGap {
    fn value(&mut self, tuning: &Tuning, random: f32) -> f32 {
        *self.0.get_or_insert_with(|| draw(tuning, random))
    }

    fn redraw(&mut self, tuning: &Tuning, random: f32) {
        self.0 = Some(draw(tuning, random));
    }

    /// The gap now, if drawn.
    pub fn current(&self) -> Option<f32> {
        self.0
    }
}

fn draw(tuning: &Tuning, random: f32) -> f32 {
    tuning.init_min_period + random.clamp(0.0, 1.0) * tuning.init_variation
}

/// A yelp of the opening exchange lasts this long, seconds.
const OPENING_YELP: f64 = 0.5;
/// A car's chase is "starting" for this long after its clock restarts, seconds.
const STARTING: f64 = 6.0;
/// In a hot chase, the starting siren screams for this long before it yelps, seconds.
const STARTING_SCREAM: f64 = 3.0;

/// One car's siren state machine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Machine {
    state: State,
    starting: bool,
    /// The chase clock (restarts when contact is lost and after each opening yelp), the wail clock and the scream
    /// clock, as times on the caller's clock.
    started: f64,
    wail_clock: f64,
    scream_clock: f64,
}

impl Machine {
    /// A machine whose clocks start at `now`.
    pub fn new(now: f64) -> Self {
        Self { state: State::Off, starting: false, started: now, wail_clock: now, scream_clock: now }
    }

    /// The machine's own state, before the light bar gate of [`Machine::update`].
    pub fn state(&self) -> State {
        self.state
    }

    /// Runs one tick at time `now` and returns the siren state the car shows. `random` is a uniform number in
    /// `0..1` for the opening gap.
    pub fn update(&mut self, now: f64, input: &Input, tuning: &Tuning, gap: &mut OpeningGap, random: f32) -> State {
        self.step(now, input, tuning, gap, random);
        if input.in_roadblock {
            self.state = State::Wail;
        }
        // The light bar is the last word: a dark bar is silent, a wrecked car's dies away.
        if input.lights_on {
            return self.state;
        }
        if input.destroyed {
            return State::Die;
        }
        State::Off
    }

    fn step(&mut self, now: f64, input: &Input, tuning: &Tuning, gap: &mut OpeningGap, random: f32) {
        if !input.in_pursuit {
            self.state = State::Off;
            return;
        }
        let mut chase_clock = true;
        match input.phase {
            Phase::Opening => {
                chase_clock = false;
                self.opening(now, tuning, gap, random);
            }
            Phase::Lost => {
                chase_clock = false;
                self.state = State::Off;
                self.started = now;
            }
            Phase::Running => {}
        }
        if !input.lights_on {
            self.state = if input.destroyed { State::Die } else { State::Off };
        }
        if chase_clock {
            self.chase(now, input, tuning);
        }
    }

    /// The opening exchange: a half-second yelp every `gap` seconds.
    fn opening(&mut self, now: f64, tuning: &Tuning, gap: &mut OpeningGap, random: f32) {
        if now - self.started > f64::from(gap.value(tuning, random)) && self.state == State::Off {
            self.wail_clock = now;
            self.state = State::Yelp;
        }
        if self.state != State::Yelp || now - self.wail_clock <= OPENING_YELP {
            return;
        }
        self.state = State::Off;
        gap.redraw(tuning, random);
        self.started = now;
    }

    /// The chase proper.
    fn chase(&mut self, now: f64, input: &Input, tuning: &Tuning) {
        let since_start = now - self.started;
        let starting = since_start < STARTING;
        if !starting && self.starting {
            self.wail_clock = now;
            self.state = if input.high_intensity { State::Yelp } else { State::Wail };
        }
        self.starting = starting;
        if starting {
            self.state = match (input.high_intensity, since_start < STARTING_SCREAM) {
                (false, _) => State::Yelp,
                (true, true) => State::Scream,
                (true, false) => State::Yelp,
            };
            return;
        }
        if input.pressing && input.in_formation {
            self.press(now, input, tuning);
            self.wail_clock = now;
            return;
        }
        let steady = now - self.wail_clock;
        let wail = f64::from(tuning.wail_period);
        if steady > wail + f64::from(tuning.max_yelp_time) {
            self.state = State::Wail;
            self.wail_clock = now;
            return;
        }
        if steady > wail {
            self.state = State::Yelp;
        }
    }

    /// A car in a pressing formation: screams for a while every so often when the chase is hot, otherwise yelps.
    fn press(&mut self, now: f64, input: &Input, tuning: &Tuning) {
        if !input.high_intensity {
            self.state = State::Yelp;
            return;
        }
        let screaming = now - self.scream_clock;
        if self.state != State::Scream && screaming > f64::from(tuning.scream_period) {
            self.state = State::Scream;
            self.scream_clock = now;
            return;
        }
        if screaming > f64::from(tuning.max_scream_time) {
            self.state = State::Yelp;
            self.scream_clock = now;
        }
    }
}
