//! The final race 1.8.1, "epic pursuit": its timed calls and door, the win and lose triggers, and the chain after the win
//! (`docs/specs/story-flow-2.md` section 5; the answers to F-11, F-31, F-32 and F-34 are in section 6, the conflict
//! C-14 in section 7, and `docs/specs/career-scripts-events.md` section 4).
//!
//! [`FinalRace`] holds the race's one-shot triggers. The caller feeds it the race state on every update and plays the
//! events it returns, in order. Each event fires once. [`after_win`] is the chain that follows the win.

/// Phone call 95, placed once when the race starts (`story-flow-2.md` section 5.3).
pub const CALL_START_ID: u16 = 95;

/// Phone call 96, placed once when the barrier call fires (`story-flow-2.md` section 5.3).
pub const CALL_BARRIER_ID: u16 = 96;

/// Seconds after the prerace entry when call 95 is placed. `RaceStartTime` is set on the prerace entry, so the clock
/// starts before the countdown (`story-flow-2.md` section 5.3, Lua `notifysimtick`).
pub const START_CALL_DELAY_SECS: f32 = 10.0;

/// Race time after which the barrier call can fire: more than this many seconds (`story-flow-2.md` section 5.3).
pub const BARRIER_CALL_DELAY_SECS: f32 = 300.0;

/// The barrier call needs the player to be further than this from the barrier area, in metres. The Lua reads "more than
/// 500 m" and the older text says "within 500 m". This follows the Lua reading until a reader checks the comparison in
/// Ghidra (conflict C-14, `story-flow-2.md` section 7).
pub const BARRIER_MIN_DISTANCE_M: f32 = 500.0;

/// The position of `barrier_area`, the GPS target (`story-flow-2.md` F-32, data of vault `epic_pursuit`). The distance
/// test takes the player's position in the same world units, read here as metres (F-32 leaves the unit open).
pub const BARRIER_AREA_POS: [f32; 3] = [-365.6, 3004.6, 188.4];

/// The win needs the car at this speed or faster, in km/h (`story-flow-2.md` section 5.3, F-31).
pub const WIN_MIN_SPEED_KMH: f32 = 180.0;

/// The win needs this direction match or better: `SimableAngle` of the player against the win trigger (F-31, the
/// constant is 0.800000011920929 in the Lua, read here as the `f32` 0.8).
pub const WIN_MIN_DIRECTION_MATCH: f32 = 0.8;

/// The NIS played as the outro of the win, at `win_nis_marker` (`story-flow-2.md` section 5.3, F-32).
pub const WIN_NIS: &str = "EndingNis04";

/// The screen opened after the win. Context bit 18 (`0x40000`) is set with it (`story-flow-2.md` section 5.4).
pub const GAME_WON_SCREEN: &str = "GameWon.fng";

/// The end-game screens in the order the counter pushes them (`story-flow-2.md` section 5.4). The counter is read, then
/// +1, and the screen for that value is pushed when `GameWon.fng` closes. Credits roll while context bit 18 is set.
pub const END_GAME_SCREENS: [&str; 5] = [
    "Credits.fng",
    "RapSheetLogin_ENDGAME.fng",
    "RapSheetLogin2_ENDGAME.fng",
    "RapSheetMain_ENDGAME.fng",
    "MainMenu_Sub.fng",
];

/// The race state the rules read on one update.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FinalRaceInput {
    /// Race time in seconds since the prerace entry (`RaceStartTime`, section 5.3).
    pub elapsed_secs: f32,
    /// The player's world position, in the units of [`BARRIER_AREA_POS`].
    pub player_pos: [f32; 3],
    /// The player's speed in km/h (`GetSimableSpeedKmh`).
    pub player_speed_kmh: f32,
    /// The player's forward direction. It is normalised before the direction match, so any length works.
    pub player_forward: [f32; 3],
    /// The player is inside `barrier_area` (the area trigger's enter event this update).
    pub in_barrier_area: bool,
    /// The player crossed the win trigger this update.
    pub crossed_win_plane: bool,
    /// The player is inside the lose box this update.
    pub in_lose_box: bool,
}

/// The place the GPS guides the player to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpsTarget {
    /// `barrier_area`, with the trigger icon on it (section 5.3).
    BarrierArea,
}

/// One effect of the final race, returned by [`FinalRace::update`] in the order it must be played.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FinalRaceEvent {
    /// Place a phone call by id: [`CALL_START_ID`] or [`CALL_BARRIER_ID`].
    PlaceCall(u16),
    /// Open `SCENERY_GROUP_DOOR` (`ENISWolrdGeometry('SCENERY_GROUP_DOOR', 0.0)`; the door is shut at prerace with
    /// 1.0). The native call behind it is not found (F-33).
    OpenDoor,
    /// Show the GPS route, and the trigger icon, to a target.
    GpsTo(GpsTarget),
    /// Hide the icon and the GPS once the player is in `barrier_area`.
    HideGps,
    /// The player won: `PlayerHasWon` is set.
    Won,
    /// The lose box was hit before the win: the camera is detached and the racer is totalled (`TotalRacer`).
    Wrecked,
    /// Play the outro NIS at `win_nis_marker`, type `Outro`. The name is [`WIN_NIS`].
    PlayOutroNis(&'static str),
    /// Bail the pursuit (`EBailPursuit(0.0, 1.0)`).
    BailPursuit,
}

/// One step of the chain after the win (`story-flow-2.md` section 5.4 and F-11), in the order it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndGameStep {
    /// `ChallengeComplete` for the player, sent when the win NIS completes (F-11).
    ChallengeComplete,
    /// `NotifyRaceFinished` for the player (F-11).
    NotifyRaceFinished,
    /// `raceover`, which reaches postrace after its 3 s message (F-11).
    RaceOverToPostRace,
    /// The career-completed flag: settings bit 14 (`0x4000`). Which code sets it first is open (F-35).
    CareerComplete,
    /// A screen of the end-game sequence: [`GAME_WON_SCREEN`], then the entries of [`END_GAME_SCREENS`].
    Screen(&'static str),
}

/// The final race's state: which one-shot triggers have fired.
///
/// Each flag guards one effect, so each event fires once. A new value is built with [`FinalRace::new`] for each run of
/// the race.
#[derive(Debug, Clone, PartialEq)]
pub struct FinalRace {
    /// The reference direction for the direction match: the win trigger's facing. The caller supplies it. How the
    /// trigger's `Rotation` (F-31) maps to this vector is native **[unconfirmed]**, and the `SimableAngle` reference is
    /// unconfirmed too, so the trigger's facing stands in for it.
    win_facing: [f32; 3],
    /// Call 95 was placed.
    start_call_sent: bool,
    /// Call 96, the door and the GPS fired.
    barrier_call_sent: bool,
    /// The GPS is on and not yet hidden by the barrier area.
    gps_on: bool,
    /// The player has won.
    won: bool,
    /// The racer was totalled by the lose box.
    wrecked: bool,
}

impl FinalRace {
    /// A race that has not started its triggers yet. `win_facing` is the reference direction for the win's direction
    /// match, the facing of the win trigger (see the field doc).
    pub fn new(win_facing: [f32; 3]) -> Self {
        Self { win_facing, start_call_sent: false, barrier_call_sent: false, gps_on: false, won: false, wrecked: false }
    }

    /// Whether the player has won.
    pub fn is_won(&self) -> bool {
        self.won
    }

    /// Runs the triggers for one update and returns the events to play, in order. The start call comes first, then the
    /// barrier call, the barrier area, the win and the lose box. A win and a lose box in the same update count the win
    /// first, so the lose box is then ignored.
    pub fn update(&mut self, input: FinalRaceInput) -> Vec<FinalRaceEvent> {
        let mut events = Vec::new();
        self.start_call(input, &mut events);
        self.barrier_call(input, &mut events);
        self.barrier_area(input, &mut events);
        self.win(input, &mut events);
        self.lose_box(input, &mut events);
        events
    }

    /// Call 95, once, at [`START_CALL_DELAY_SECS`] (section 5.3).
    fn start_call(&mut self, input: FinalRaceInput, events: &mut Vec<FinalRaceEvent>) {
        if self.start_call_sent {
            return;
        }
        if input.elapsed_secs < START_CALL_DELAY_SECS {
            return;
        }
        self.start_call_sent = true;
        events.push(FinalRaceEvent::PlaceCall(CALL_START_ID));
    }

    /// Call 96, the door and the GPS, once: race time over [`BARRIER_CALL_DELAY_SECS`] and the player further than
    /// [`BARRIER_MIN_DISTANCE_M`] from the barrier area (section 5.3, conflict C-14).
    fn barrier_call(&mut self, input: FinalRaceInput, events: &mut Vec<FinalRaceEvent>) {
        if self.barrier_call_sent {
            return;
        }
        if input.elapsed_secs <= BARRIER_CALL_DELAY_SECS {
            return;
        }
        if distance(input.player_pos, BARRIER_AREA_POS) <= BARRIER_MIN_DISTANCE_M {
            return;
        }
        self.barrier_call_sent = true;
        self.gps_on = true;
        events.extend([
            FinalRaceEvent::PlaceCall(CALL_BARRIER_ID),
            FinalRaceEvent::OpenDoor,
            FinalRaceEvent::GpsTo(GpsTarget::BarrierArea),
        ]);
    }

    /// Entering the barrier area hides the GPS, but only once the GPS is on (section 5.3, F-32).
    fn barrier_area(&mut self, input: FinalRaceInput, events: &mut Vec<FinalRaceEvent>) {
        if !self.gps_on {
            return;
        }
        if !input.in_barrier_area {
            return;
        }
        self.gps_on = false;
        events.push(FinalRaceEvent::HideGps);
    }

    /// The win: the win trigger crossed at [`WIN_MIN_SPEED_KMH`] or faster with a direction match of
    /// [`WIN_MIN_DIRECTION_MATCH`] or better. Below either value the handler returns with no change (F-31).
    fn win(&mut self, input: FinalRaceInput, events: &mut Vec<FinalRaceEvent>) {
        if self.won {
            return;
        }
        if !input.crossed_win_plane {
            return;
        }
        if input.player_speed_kmh < WIN_MIN_SPEED_KMH {
            return;
        }
        if self.direction_match(input.player_forward) < WIN_MIN_DIRECTION_MATCH {
            return;
        }
        self.won = true;
        events.extend([FinalRaceEvent::Won, FinalRaceEvent::PlayOutroNis(WIN_NIS), FinalRaceEvent::BailPursuit]);
    }

    /// The lose box before the win totals the racer. After the win it is ignored (section 5.3, F-32).
    fn lose_box(&mut self, input: FinalRaceInput, events: &mut Vec<FinalRaceEvent>) {
        if self.won {
            return;
        }
        if self.wrecked {
            return;
        }
        if !input.in_lose_box {
            return;
        }
        self.wrecked = true;
        events.push(FinalRaceEvent::Wrecked);
    }

    /// The cosine of the angle between the player's forward and the win trigger's facing, in the range -1 to 1. A zero
    /// vector matches nothing.
    fn direction_match(&self, forward: [f32; 3]) -> f32 {
        let norm = length(forward) * length(self.win_facing);
        if norm <= 0.0 {
            return 0.0;
        }
        dot(forward, self.win_facing) / norm
    }
}

/// The chain after the win, in the order it runs (`story-flow-2.md` section 5.4, F-11, F-34, F-35): the race-over and
/// post-race steps, the career-completed flag, `GameWon.fng`, then the end-game screens.
///
/// The call returns the whole chain. The caller plays each step as the earlier one completes. Where context bit 18 is
/// cleared, and how `RapSheetMain_ENDGAME` ends, are open (F-34).
pub fn after_win() -> Vec<EndGameStep> {
    let mut steps = vec![
        EndGameStep::ChallengeComplete,
        EndGameStep::NotifyRaceFinished,
        EndGameStep::RaceOverToPostRace,
        EndGameStep::CareerComplete,
        EndGameStep::Screen(GAME_WON_SCREEN),
    ];
    steps.extend(END_GAME_SCREENS.iter().copied().map(EndGameStep::Screen));
    steps
}

/// The distance between two points, in the units of the points.
fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    length([a[0] - b[0], a[1] - b[1], a[2] - b[2]])
}

/// The length of a vector.
fn length(v: [f32; 3]) -> f32 {
    dot(v, v).sqrt()
}

/// The dot product of two vectors.
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[cfg(test)]
#[path = "final_race_tests.rs"]
mod tests;
