//! The rules of a mod car's own effects, apart from any sound: how far the turbo has spooled, how the whistle loops
//! are blended, and what a lifted throttle sets off. The 2012 game decides these in its sound graphs, which are not
//! decoded (docs/formats/chameleon-sound.md); these are the game's own simple rules, chosen to sound plausible.

use std::f32::consts::FRAC_PI_2;

/// The turbo spools up to its target in about this many seconds (the time constant of an exponential approach).
const SPOOL_UP_SECONDS: f32 = 0.9;
/// And winds down this fast once the throttle is lifted.
const SPOOL_DOWN_SECONDS: f32 = 0.6;
/// The whistle's top level: a whistle under the engine should not cover it.
const WHINE_LEVEL: f32 = 0.6;
/// The whistle plays this much slower with the turbo stopped, and this much faster at full spool.
const WHINE_RATE: (f32, f32) = (0.8, 1.4);
/// The turbo vents when the throttle is lifted with at least this much spool.
const BLOWOFF_MIN_SPOOL: f32 = 0.3;
/// A vented turbo is left with this fraction of its spool.
pub const SPOOL_AFTER_BLOWOFF: f32 = 0.4;

/// A small repeatable random-number generator (xorshift): the same car always makes the same choices.
#[derive(Debug, Clone)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed | 1)
    }

    /// A number from 0 up to, not including, 1.
    pub fn unit(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }

    /// A number from 0 up to, not including, `n` (0 for an `n` of 0).
    pub fn below(&mut self, n: usize) -> usize {
        ((self.unit() * n as f32) as usize).min(n.saturating_sub(1))
    }
}

/// The spool (0 to 1) after `dt` seconds: it chases a target of the throttle times how hard the engine is turning, up
/// slowly and down faster.
pub fn spool_step(spool: f32, throttle: f32, rpm_pct: f32, dt: f32) -> f32 {
    let target = throttle.clamp(0.0, 1.0) * (0.25 + 0.75 * rpm_pct.clamp(0.0, 1.0));
    let seconds = if target > spool { SPOOL_UP_SECONDS } else { SPOOL_DOWN_SECONDS };
    let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
    (spool + (target - spool) * (1.0 - (-dt / seconds).exp())).clamp(0.0, 1.0)
}

/// What one whistle loop should sound like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhineMix {
    pub gain: f32,
    pub rate: f32,
}

/// The mix of `layers` whistle loops at `spool`: silent with the turbo stopped, louder and higher as it spools, and
/// the loops laid out along the spool in the order given, each fading into the next with constant power.
pub fn whine_mix(layers: usize, spool: f32) -> Vec<WhineMix> {
    let spool = spool.clamp(0.0, 1.0);
    let total = spool.powf(1.5) * WHINE_LEVEL;
    let rate = WHINE_RATE.0 + (WHINE_RATE.1 - WHINE_RATE.0) * spool;
    let mut mix = vec![WhineMix { gain: 0.0, rate }; layers];
    match layers {
        0 => {}
        1 => mix[0].gain = total,
        _ => {
            let position = spool * (layers - 1) as f32;
            let below = (position as usize).min(layers - 2);
            let t = position - below as f32;
            mix[below].gain = (total * (t * FRAC_PI_2).cos()).max(0.0);
            mix[below + 1].gain = (total * (t * FRAC_PI_2).sin()).max(0.0);
        }
    }
    mix
}

/// One bang after a backfire.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pop {
    /// Seconds after the lift.
    pub delay: f32,
    pub volume: f32,
}

/// What a throttle lifted at high engine speed sets off. Each part is played only if the car has the sound for it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LiftOff {
    /// The turbo vents, at this level.
    pub blowoff: Option<f32>,
    /// The exhaust backfires, at this level.
    pub backfire: Option<f32>,
    /// Bangs that follow, in time order.
    pub pops: Vec<Pop>,
}

/// How likely a lift-off at `rpm_pct` is to backfire: a few percent near idle, a quarter at the limiter (measured against
/// a drive of six shifts, the earlier 20 to 70 percent was a bang at nearly every lift).
pub fn backfire_chance(rpm_pct: f32) -> f32 {
    let rpm_pct = rpm_pct.clamp(0.0, 1.0);
    0.03 + 0.22 * rpm_pct * rpm_pct
}

/// The chance that a lift-off with no backfire still pops once or twice.
const LONE_POP_CHANCE: f32 = 0.06;

/// The effects of a lift-off at `spool` and `rpm_pct`. The faster the engine turns the likelier it is to backfire; a
/// backfire is followed by one to three pops, and a lift-off without one sometimes has one or two pops of its own.
pub fn lift_off(spool: f32, rpm_pct: f32, rng: &mut Rng) -> LiftOff {
    let rpm_pct = rpm_pct.clamp(0.0, 1.0);
    let blowoff = (spool >= BLOWOFF_MIN_SPOOL).then(|| 0.5 + 0.5 * spool.min(1.0));
    let chance = backfire_chance(rpm_pct);
    let roll = rng.unit();
    let backfire = (roll < chance).then_some(0.5 + 0.5 * rpm_pct);
    let count = match roll {
        r if r < chance => 1 + rng.below(3),
        r if r < chance + LONE_POP_CHANCE => 1 + rng.below(2),
        _ => 0,
    };
    LiftOff { blowoff, backfire, pops: pops_of(count, rng) }
}

/// `count` pops in time order, the first 0.12 s after the bang and each 0.05 to 0.25 s after the one before.
fn pops_of(count: usize, rng: &mut Rng) -> Vec<Pop> {
    let mut delay = 0.12;
    let mut pops = Vec::with_capacity(count);
    for _ in 0..count {
        pops.push(Pop { delay, volume: 0.4 + 0.4 * rng.unit() });
        delay += 0.05 + 0.2 * rng.unit();
    }
    pops
}

/// The pops that follow a bang the overrun controller decided (`[car_sound.overrun]`): one to three, as after a core
/// backfire. The overrun table does not drive them; they keep their core meaning (docs/modding/car-sound.md).
pub fn pops_after_bang(rng: &mut Rng) -> Vec<Pop> {
    let count = 1 + rng.below(3);
    pops_of(count, rng)
}

/// Most bangs that wait to light the tail pipes at once (a lift-off makes four at most).
const MAX_BANGS: usize = 16;

/// The bangs of lift-offs that have not lit the tail pipes yet. The game's own cars have a sputter module whose pops light
/// the pipes (`take_sputter_pops`); a mod car has none, so its backfire and pops, which [`lift_off`] decides, do it: each
/// bang waits as long as its sound is held back, then is due once.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bangs {
    /// Seconds until each waiting bang.
    waiting: Vec<f32>,
    /// Bangs whose time has come since the last [`Bangs::take`].
    due: u32,
}

impl Bangs {
    /// A bang `delay` seconds from now.
    pub fn queue(&mut self, delay: f32) {
        if self.waiting.len() >= MAX_BANGS {
            return;
        }
        let delay = match delay.is_finite() {
            true => delay.max(0.0),
            false => 0.0,
        };
        self.waiting.push(delay);
    }

    /// Let `dt` seconds pass: the bangs whose time is up become due.
    pub fn advance(&mut self, dt: f32) {
        let dt = match dt.is_finite() {
            true => dt.max(0.0),
            false => 0.0,
        };
        let before = self.waiting.len();
        for left in &mut self.waiting {
            *left -= dt;
        }
        self.waiting.retain(|left| *left > 0.0);
        self.due += (before - self.waiting.len()) as u32;
    }

    /// The bangs that became due since the last call.
    pub fn take(&mut self) -> u32 {
        std::mem::take(&mut self.due)
    }
}

#[cfg(test)]
mod tests;
