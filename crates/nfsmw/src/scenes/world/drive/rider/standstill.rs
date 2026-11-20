//! The standstill: the rider puts his feet down as the bike stops, waits with them on the ground and takes them back up as it
//! pulls away (`docs/specs/ride6-rider-standstill.md`).
//!
//! The animation blueprint's state machine `Main_SM` has the states `Standing`, `Racing`, `Start` and `Stop`. Its functions
//! `Standing/StartStop_*_W_I_P` make two smoothed 0 to 1 values from whether the bike is moving (`VehicleSpeed` above
//! `StartStop Speed Treshold`, 6 km/h): `StartClip_Explicit_Normalized` (going up while it moves, at `Start Anim Speed`) and
//! `StopClip_Explicit_Normalized` (going up while it does not, at `Stop Anim Speed`). The start clip plays from `Start Clip Start`
//! by that value over `Start Clip Start` to `Start Clip End` seconds, the stop clip likewise. The `Start` state hands over to
//! `Racing` when its value reaches `Racing blend start`, 0.85; the others follow from the values reaching 0 and 1.
//! `IK_Alpha_StartStop/Ground_*_W_I_P` then say, from the seconds the clips are at, whether each foot is held on its peg
//! (`LeftFoot_IK_onFootboard`, `RightFoot_IK_onFootboard`), smoothed at the foot's `IK Smooth Speed`.
//!
//! Which states and which signals are the blueprint's and which are choices is in the spec: the transitions of the state machine
//! are in bytecode that names its conditions only by the variables above, so the conditions here are read from those.

use super::script::Constant;
use super::style::Stand;

/// How near to 0 and 1 the smoothed values count as there: the smoother stops short of its target by a hair.
const EDGE: f32 = 1e-3;

/// How fast the pose crosses between riding and the standstill (per second). A choice: the transition times of the state machine
/// are in cooked data that was not read.
const FADE_RATE: f32 = 6.0;

/// The states of `Main_SM` that matter for the feet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    /// `Racing`: the bike is moving, the feet are on the pegs.
    #[default]
    Riding,
    /// `Stop`: the stop clip puts the feet down.
    Stopping,
    /// `Standing`: the rider waits.
    Standing,
    /// `Start`: the start clip takes the feet back up.
    Starting,
}

/// Which feet go down at a standstill, as the graph's `stand_leg` parameter takes it (the blueprint's `StandingLeg` 0, -1 and 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Leg {
    #[default]
    Both,
    Left,
    Right,
}

impl Leg {
    fn index(self) -> usize {
        match self {
            Self::Both => 0,
            Self::Left => 1,
            Self::Right => 2,
        }
    }
}

/// The foot a riding style puts down at a standstill (`START_LEG` of the `RIDING_STYLE` presets: balanced the right foot, shoulders
/// out the left, old school both feet).
pub fn leg_of(riding_style: u8) -> Leg {
    match riding_style {
        0 => Leg::Right,
        1 => Leg::Left,
        _ => Leg::Both,
    }
}

/// What the graph and the legs read.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Signals {
    /// How much of the pose is the standstill, 0 to 1.
    pub weight: f32,
    /// 0 the stop clip, 1 the standing clip, 2 the start clip.
    pub state: f32,
    /// 0 both feet, 1 the left, 2 the right.
    pub leg: f32,
    /// Seconds into the stop clip and into the start clip.
    pub stop_time: f32,
    pub start_time: f32,
    /// How firmly each foot (left, right) is held on its peg, 0 (let go) to 1 (on it).
    pub pegs: [f32; 2],
}

/// The standstill's state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Standstill {
    stand: Option<Stand>,
    leg: Leg,
    phase: Phase,
    /// The first step decides whether the bike starts standing or riding.
    started: bool,
    start_norm: Constant,
    stop_norm: Constant,
    weight: Constant,
    pegs: [Constant; 2],
    /// Whether the start value was not falling in the last step (`StartStop_isStart`).
    rising: bool,
}

impl Standstill {
    /// A standstill with the numbers `stand` (`None`: the rider never puts his feet down) and the feet `leg`.
    pub fn new(stand: Option<Stand>, leg: Leg) -> Self {
        Self { stand, leg, rising: true, pegs: [Constant::at(1.0); 2], ..Self::default() }
    }

    /// Put the other foot down from now on.
    pub fn set_leg(&mut self, leg: Leg) {
        self.leg = leg;
    }

    /// Advance by `dt` seconds with the bike moving at `speed_kmh` along itself (negative backwards: a bike that rolls back is not
    /// moving; the reverse walk takes the pose over from the standstill in the graph).
    pub fn step(&mut self, speed_kmh: f32, dt: f32) -> Signals {
        let Some(stand) = &self.stand else { return Signals { pegs: [1.0; 2], ..Signals::default() } };
        let (threshold, racing_at, leg) = (stand.speed_kmh, stand.racing_at, stand.legs[self.leg.index()]);
        let moving = speed_kmh.is_finite() && speed_kmh > threshold;
        if !self.started {
            // A bike that is standing at the start of the scene stands; one that is already moving rides.
            self.started = true;
            self.phase = if moving { Phase::Riding } else { Phase::Standing };
            self.start_norm = Constant::at(f32::from(moving));
            self.stop_norm = Constant::at(f32::from(!moving));
            self.weight = Constant::at(f32::from(!moving));
            self.pegs = [Constant::at(f32::from(moving)); 2];
        }
        let before = self.start_norm.value();
        let start = self.start_norm.step(f32::from(moving), leg.start_rate, dt);
        let stop = self.stop_norm.step(f32::from(!moving), leg.stop_rate, dt);
        // `StartStop_isStart`: the derivative of the start value is not negative.
        self.rising = start >= before;
        self.phase = match (self.phase, moving) {
            (Phase::Riding, false) => Phase::Stopping,
            (Phase::Stopping, _) if stop >= 1.0 - EDGE => Phase::Standing,
            (Phase::Stopping, true) if stop <= EDGE => Phase::Riding,
            (Phase::Standing, true) => Phase::Starting,
            (Phase::Starting, _) if start >= racing_at => Phase::Riding,
            (Phase::Starting, false) if start <= EDGE => Phase::Standing,
            (phase, _) => phase,
        };
        let down = self.phase != Phase::Riding;
        let weight = self.weight.step(f32::from(down), FADE_RATE, dt);
        let at = |window: [f32; 2], value: f32| window[0] + value.clamp(0.0, 1.0) * (window[1] - window[0]);
        let (stop_time, start_time) = (at(leg.stop_seconds, stop), at(leg.start_seconds, start));
        let pegs = [
            self.peg(0, leg.left_ik, (stop_time, start_time), leg.ik_rate[0], dt),
            self.peg(1, leg.right_ik, (stop_time, start_time), leg.ik_rate[1], dt),
        ];
        let state = match self.phase {
            Phase::Stopping => 0.0,
            Phase::Standing | Phase::Riding => 1.0,
            Phase::Starting => 2.0,
        };
        Signals { weight, state, leg: self.leg.index() as f32, stop_time, start_time, pegs }
    }

    /// `LeftFoot_IK_onFootboard_Smooth` and its right twin: whether the foot is held on its peg, `ik` being the foot's four
    /// seconds (off the ground in the start clip, on the peg in the start clip, off the peg in the stop clip, on the ground in the stop
    /// clip). A foot with none stays on its peg.
    fn peg(&mut self, foot: usize, ik: Option<[f32; 4]>, (stop, start): (f32, f32), rate: f32, dt: f32) -> f32 {
        let Some([_, on_footboard, off_footboard, _]) = ik else { return 1.0 };
        let held = match self.rising {
            true => start >= on_footboard,
            false => stop < off_footboard,
        };
        self.pegs[foot].step(f32::from(held), rate, dt)
    }
}

#[cfg(test)]
#[path = "standstill_tests.rs"]
mod tests;
