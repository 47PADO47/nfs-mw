//! The machine's own state: the speech pursuit state and focus, the timers, the counters and what it remembers of the
//! player's heading. The flows and the roster read and write it through [`Cx`](super::Cx).

use glam::Vec3;

use super::input::Impact;
use super::params::Direction;
use crate::infraction::Infraction;

/// A small seeded generator for the cosmetic choices (which cop speaks, a coin for a variant).
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1 }
    }

    fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    /// A uniform number in `0..1`.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// A uniform number in `0..n` (0 for an empty range).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_f32() * n as f32) as usize % n
    }

    /// The original's coin: true when a uniform number exceeds one half.
    pub fn coin(&mut self) -> bool {
        self.next_f32() > 0.5
    }

    /// Shuffles `items` in place the way the original does (each slot swaps with a random one).
    pub fn shuffle<T: Copy>(&mut self, items: &mut [T]) {
        for i in 0..items.len() {
            let j = self.below(items.len());
            items.swap(i, j);
        }
    }
}

/// The speech machine's view of the pursuit (not the same as the pursuit's own status).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PursuitState {
    /// The pursuit is on and a cop (or the helicopter, or a roadblock) has the player in sight.
    Active,
    /// No cop sees the player, or the pursuit is re-acquiring or cooling down.
    Searching,
    /// No pursuit for the player.
    Inactive,
    /// Only an AI racer is pursued.
    OtherTarget,
}

/// Which flow may talk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// The opening exchange of a chase.
    Pursuit,
    /// Normal chase commentary.
    Strategy,
    /// Contact lost: the quadrant sequence.
    Lost,
    /// Busted or abandoned: everything stops.
    Terminal,
}

/// How far the lost-contact fiction has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quadrant {
    Initial,
    Forming,
    Fiction1,
    Fiction2,
    Expired,
    Reset,
}

/// What the last observation was (dispatch words its escalation after it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Observation {
    #[default]
    None,
    CopHitCop,
    CopHitTraffic,
    CopHitSuspect(Impact),
    CopHitWorld,
    SuspectHitWorld,
    SuspectHitSuspect,
    SuspectHitTraffic,
    SuspectHitGuardrail,
    SuspectHitTrain,
    SuspectHitSemi,
    SuspectHitGasStation,
    SuspectHitSpikeBelt,
    Spunout,
    Airborne,
    Rollover,
}

/// A heading as the machine reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Heading {
    pub direction: Option<Direction>,
    pub road: u16,
}

/// The last time the player passed a cop faster than it could follow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blowby {
    pub distance: f32,
    pub speed: f32,
    pub at: f32,
}

impl Default for Blowby {
    fn default() -> Self {
        Self { distance: 32767.0, speed: 0.0, at: f32::NEG_INFINITY }
    }
}

/// The flags of the original's `SoundAI`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    pub low_speed_timer: bool,
    pub cops_ahead: bool,
    pub heli_intro_requested: bool,
    pub busted: bool,
    pub disp911_active: bool,
    pub setup_restarted: bool,
    pub cops_immune: bool,
    pub racers_near: bool,
    pub pursuit_expired: bool,
}

/// All the machine remembers between steps.
#[derive(Debug, Clone)]
pub struct Core {
    pub state: PursuitState,
    pub focus: Focus,
    pub quadrant: Quadrant,
    pub flags: Flags,
    pub t_pursuit_start: f32,
    pub t_since_last_pursuit: f32,
    pub t_no_los: f32,
    pub t_los: f32,
    pub t_low_speed: f32,
    pub t_last_crashed: f32,
    /// When the player last rammed a cop, whoever the cop was (the original's global, not the roster's).
    pub t_last_nailed: f32,
    pub t_last_cop_nailed: f32,
    /// Seconds the pursuit has lasted, or -1 when none runs.
    pub duration: f32,
    pub time_since_last_chase: f32,
    pub pursuit_dist: f32,
    pub los_count: usize,
    pub cops_in_view: usize,
    pub active_cops: usize,
    pub blowby: Blowby,
    pub infraction: Option<Infraction>,
    pub traffic_hits_911: i32,
    pub cost_911: i32,
    pub smoothed_forward: Vec3,
    pub direction_tracking: Option<Direction>,
    pub t_direction: f32,
    pub current: [Heading; 2],
    pub last_known: Heading,
    pub prev_heat: f32,
    pub last_observation: Observation,
    /// The player is in a tunnel (the helicopter cannot see into one).
    pub tunnel: bool,
}

impl Core {
    pub fn new() -> Self {
        Self {
            state: PursuitState::Inactive,
            focus: Focus::Pursuit,
            quadrant: Quadrant::Reset,
            flags: Flags::default(),
            t_pursuit_start: 0.0,
            t_since_last_pursuit: f32::NEG_INFINITY,
            t_no_los: f32::NEG_INFINITY,
            t_los: f32::NEG_INFINITY,
            t_low_speed: f32::NEG_INFINITY,
            t_last_crashed: f32::NEG_INFINITY,
            t_last_nailed: f32::NEG_INFINITY,
            t_last_cop_nailed: f32::NEG_INFINITY,
            duration: -1.0,
            time_since_last_chase: 0.0,
            pursuit_dist: 0.0,
            los_count: 0,
            cops_in_view: 0,
            active_cops: 0,
            blowby: Blowby::default(),
            infraction: None,
            traffic_hits_911: 0,
            cost_911: 0,
            smoothed_forward: Vec3::ZERO,
            direction_tracking: None,
            t_direction: f32::NEG_INFINITY,
            current: [Heading::default(); 2],
            last_known: Heading::default(),
            prev_heat: 1.0,
            last_observation: Observation::None,
            tunnel: false,
        }
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}
