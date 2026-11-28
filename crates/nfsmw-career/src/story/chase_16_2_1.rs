//! The scripted triggers of the two D-Day races, `16.1.0` and `16.2.1`, as a state machine (`docs/specs/story-flow.md`
//! section 2.4; `docs/specs/career-scripts-events.md` section 2.2; F-15 and F-16 in `docs/specs/story-flow-2.md` section
//! 6). Each race has its own copy of the trigger handler, so the rules differ by [`ChaseScript`]:
//!
//! The caller feeds one [`ChaseInput`] per frame and plays the [`ChaseEvent`]s it gets back. The machine owns the
//! timers that the scripts start on each trigger, and each event is emitted once:
//!
//! - the call trigger requests phone call 10. In `16.1.0` it waits for the call to load (`WaitForCallLoad`); in `16.2.1`
//!   it starts the backup sabotage instead, 15 s after the trigger (`secs_since_call_trigger` is the caller's clock from
//!   that trigger);
//! - the kill trigger sabotages at once, if the engine is not sabotaged yet;
//! - when a waited-for call starts, an 8 s fade timer starts; when it fires the screen fades on, and 3 s later the race
//!   ends (11 s after the call started, F-16);
//! - when the engine blows, a 1 s timer ends the race (F-16). Whichever end fires first ends the race, and nothing is
//!   emitted after it.
//!
//! The timers on the call and the engine count the frames' `dt`. A clock starts on the frame of its event, so the
//! event's own frame adds no time and the next frames do.

/// What a race's copy of the trigger handler does at the call trigger **[verified]** (the Lua chunks of `gameplay.bin`:
/// `16.1.0`'s copy sets `WaitForCallLoad` and calls `ECellCall(10)`; `16.2.1`'s copy starts the `backup` timer and calls
/// `ECellCall(10)`, and only it handles `kill_engine`, `cop_spawn_trigger` and `bail_trigger`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChaseScript {
    /// The call is waited for: its start runs the fade and kill timers that end the race.
    pub call_ends_race: bool,
    /// The call trigger starts the 15 s backup sabotage.
    pub backup_sabotage: bool,
}

/// `16.1.0`: the call from Mia ends the race 11 s after it starts. No sabotage.
pub const OPENING_SCRIPT: ChaseScript = ChaseScript { call_ends_race: true, backup_sabotage: false };
/// `16.2.1`: the call from Mia comes with the backup sabotage; the blown engine ends the race.
pub const CHASE_SCRIPT: ChaseScript = ChaseScript { call_ends_race: false, backup_sabotage: true };

/// The script of the race with event id `event`, for the two races that have one.
pub fn script_of(event: &str) -> Option<ChaseScript> {
    match event {
        "16.1.0" => Some(OPENING_SCRIPT),
        "16.2.1" => Some(CHASE_SCRIPT),
        _ => None,
    }
}

/// The sabotage level of both sabotage paths (section 2.4).
pub const SABOTAGE_LEVEL: u8 = 5;
/// The phone call the call trigger requests (`ECellCall` 10).
pub const CHASE_CALL_ID: u32 = 10;
/// The backup sabotage, measured from the call trigger.
pub const BACKUP_SABOTAGE_SECS: f32 = 15.0;
/// The fade timer, measured from the call start: the screen fades on when it fires.
pub const FADE_ON_SECS: f32 = 8.0;
/// The race end on the call path, measured from the call start: the fade (8 s) and the kill timer (3 s).
pub const CALL_RACE_END_SECS: f32 = 11.0;
/// The race end on the engine path, measured from the engine blowing.
pub const ENGINE_BLOWN_END_SECS: f32 = 1.0;
/// The number of cops the cop trigger spawns.
pub const COP_SPAWN_COUNT: u8 = 2;
/// The bail pursuit parameters (`EBailPursuit(1.0, 1.0)`).
pub const BAIL_PURSUIT_PARAMS: (f32, f32) = (1.0, 1.0);

/// The cops the cop trigger spawns (`copmidsize`, F-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopKind {
    /// A mid-size police car.
    MidSize,
}

/// What the game reports this frame. The event flags are true only in the frame the event happens.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ChaseInput {
    /// Seconds since the player entered the call trigger, or `None` before that. The caller keeps this clock.
    pub secs_since_call_trigger: Option<f32>,
    /// The player entered `kill_engine`.
    pub at_kill_engine: bool,
    /// The player entered `cop_spawn_trigger`.
    pub at_cop_spawn: bool,
    /// The player entered `bail_trigger`.
    pub at_bail: bool,
    /// `notifycellcallstarted`: the requested call has started.
    pub call_started: bool,
    /// `notifyengineblown`: the player car's engine blew.
    pub engine_blown: bool,
    /// The frame time in seconds.
    pub dt: f32,
}

/// What the chase asks the game to do. Each one is emitted once.
#[derive(Debug, Clone, PartialEq)]
pub enum ChaseEvent {
    /// `SabotageEngine` at the level.
    SabotageEngine {
        /// The sabotage level.
        level: u8,
    },
    /// `ECellCall` with the call id.
    PlaceCall {
        /// The call id.
        id: u32,
    },
    /// The cops are switched on.
    CopsOn,
    /// Spawn cops of a kind at the cop spawn points.
    SpawnCops {
        /// How many.
        count: u8,
        /// Which kind.
        kind: CopKind,
    },
    /// `EDispIntroRace()` with no argument.
    DispIntroRace,
    /// `EBailPursuit(a, b)`.
    BailPursuit {
        /// The first parameter.
        a: f32,
        /// The second parameter.
        b: f32,
    },
    /// `Game.NoNewPursuitsOrCops()`.
    NoNewPursuits,
    /// The screen fades on (the 8 s fade timer fired).
    FadeOn,
    /// The screen fades out and the race goes to postrace. Sent by both end paths.
    FadeOut,
    /// The race goes to postrace (`postrace`).
    PostRace,
}

/// The chase's state. Build it with [`Chase::for_script`] and call [`Chase::update`] once per frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Chase {
    script: ChaseScript,
    sabotaged: bool,
    call_placed: bool,
    waiting_for_call: bool,
    /// Seconds since the call started, once it has.
    call_clock: Option<f32>,
    faded_on: bool,
    /// Seconds since the engine blew, once it has.
    blow_clock: Option<f32>,
    cops_on: bool,
    bailed: bool,
    finished: bool,
}

impl Default for Chase {
    fn default() -> Chase {
        Chase::new()
    }
}

impl Chase {
    /// The chase of `16.2.1` before any trigger.
    pub fn new() -> Chase {
        Chase::for_script(CHASE_SCRIPT)
    }

    /// The race's triggers under `script`, before any trigger.
    pub fn for_script(script: ChaseScript) -> Chase {
        Chase {
            script,
            sabotaged: false,
            call_placed: false,
            waiting_for_call: false,
            call_clock: None,
            faded_on: false,
            blow_clock: None,
            cops_on: false,
            bailed: false,
            finished: false,
        }
    }

    /// True once the race has gone to postrace. No later update emits anything.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Whether the opening race has handed both cars to AI for the fade.
    pub fn forces_ai_control(&self) -> bool {
        self.faded_on && !self.finished
    }

    /// Advances the chase by one frame and returns what it asks for, in order.
    pub fn update(&mut self, input: ChaseInput) -> Vec<ChaseEvent> {
        let mut out = Vec::new();
        if self.finished {
            return out;
        }
        self.tick(input.dt);
        self.on_triggers(input, &mut out);
        self.start_clocks(input);
        self.fire_timers(&mut out);
        out
    }

    /// Advances the running clocks by the frame time.
    fn tick(&mut self, dt: f32) {
        if let Some(clock) = self.call_clock.as_mut() {
            *clock += dt;
        }
        if let Some(clock) = self.blow_clock.as_mut() {
            *clock += dt;
        }
    }

    /// The triggers of this frame: the call request, the two sabotage paths, the cops and the bail.
    fn on_triggers(&mut self, input: ChaseInput, out: &mut Vec<ChaseEvent>) {
        if input.at_kill_engine {
            self.sabotage(out);
        }
        if input.secs_since_call_trigger.is_some() && !self.call_placed {
            self.call_placed = true;
            self.waiting_for_call = self.script.call_ends_race;
            out.push(ChaseEvent::PlaceCall { id: CHASE_CALL_ID });
        }
        let backup_due = input.secs_since_call_trigger.is_some_and(|secs| secs >= BACKUP_SABOTAGE_SECS);
        if self.script.backup_sabotage && backup_due {
            self.sabotage(out);
        }
        if input.at_cop_spawn {
            self.spawn_cops(out);
        }
        if input.at_bail {
            self.bail(out);
        }
    }

    /// Sabotages the engine once.
    fn sabotage(&mut self, out: &mut Vec<ChaseEvent>) {
        if self.sabotaged {
            return;
        }
        self.sabotaged = true;
        out.push(ChaseEvent::SabotageEngine { level: SABOTAGE_LEVEL });
    }

    /// The cop trigger: cops on, then two mid-size cops, once.
    fn spawn_cops(&mut self, out: &mut Vec<ChaseEvent>) {
        if self.cops_on {
            return;
        }
        self.cops_on = true;
        out.push(ChaseEvent::CopsOn);
        out.push(ChaseEvent::SpawnCops { count: COP_SPAWN_COUNT, kind: CopKind::MidSize });
    }

    /// The bail trigger: the intro race display, the bail pursuit, then no new pursuits, once.
    fn bail(&mut self, out: &mut Vec<ChaseEvent>) {
        if self.bailed {
            return;
        }
        self.bailed = true;
        let (a, b) = BAIL_PURSUIT_PARAMS;
        out.push(ChaseEvent::DispIntroRace);
        out.push(ChaseEvent::BailPursuit { a, b });
        out.push(ChaseEvent::NoNewPursuits);
    }

    /// Starts the call and engine clocks on the edges of their events. A call start counts only while the call is
    /// waited for, and an engine blow starts its clock once.
    fn start_clocks(&mut self, input: ChaseInput) {
        if input.call_started && self.waiting_for_call {
            self.waiting_for_call = false;
            self.call_clock = Some(0.0);
        }
        if input.engine_blown && self.blow_clock.is_none() {
            self.blow_clock = Some(0.0);
        }
    }

    /// Fires the timers that have run out. The engine path is checked first; either end ends the race.
    fn fire_timers(&mut self, out: &mut Vec<ChaseEvent>) {
        if self.blow_clock.is_some_and(|clock| clock >= ENGINE_BLOWN_END_SECS) {
            return self.end(out);
        }
        let Some(clock) = self.call_clock else {
            return;
        };
        if clock >= FADE_ON_SECS && !self.faded_on {
            self.faded_on = true;
            out.push(ChaseEvent::FadeOn);
        }
        if clock >= CALL_RACE_END_SECS {
            self.end(out);
        }
    }

    /// The race goes to postrace. Nothing else is emitted afterwards.
    fn end(&mut self, out: &mut Vec<ChaseEvent>) {
        self.finished = true;
        out.push(ChaseEvent::FadeOut);
        out.push(ChaseEvent::PostRace);
    }
}

#[cfg(test)]
#[path = "chase_16_2_1_tests.rs"]
mod tests;
