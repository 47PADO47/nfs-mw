//! The node functions the module code calls. Spec: `docs/specs/aems.md` §2 to §6. Each takes the offset of its node
//! in the instance image and returns the node's output.

mod logic;
mod math;
mod player;
mod signal;

pub(crate) use player::teardown;

use crate::error::{Error, Result};
use crate::host::Host;
use crate::mem::Mem;

/// A small seeded generator (xorshift32): equal seeds give equal sound.
#[derive(Debug, Clone)]
pub(crate) struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9 } else { seed })
    }

    pub fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x >> 1
    }
}

/// What a node function can reach.
pub(crate) struct Cx<'a> {
    pub mem: &'a mut Mem,
    /// The tick period, milliseconds.
    pub period: f32,
    pub rng: &'a mut Rng,
    pub host: &'a mut dyn Host,
    /// Offsets of the player nodes and of the class-controller nodes.
    pub players: &'a [usize],
    pub controllers: &'a [usize],
    pub destroyed: &'a mut bool,
}

/// A float to an integer the way the PC's `fistp` does it: to nearest, ties to even.
pub(crate) fn ftoi(x: f32) -> i32 {
    x.round_ties_even() as i32
}

/// Calls node function `id` on the node at `p`.
pub(crate) fn call(cx: &mut Cx<'_>, id: u32, p: usize) -> Result<i32> {
    let m = &mut *cx.mem;
    Ok(match id {
        0 => logic::flag(m, p + 0x10),
        1 => m.i32(p + 0x14),
        2 => m.i32(p + 0x18),
        3 => logic::flag(m, p),
        4 => return player::destroy(cx, p),
        5 | 37 | 39 => logic::function_flag(m, id, p),
        6 => logic::counter(m, p),
        7 => logic::random(m, cx.rng, p),
        8 => logic::random_shuffle(m, cx.rng, p),
        9 => logic::random_weighted(m, cx.rng, p),
        10 => logic::range_trigger(m, p),
        11 => logic::delay_trigger(m, cx.period, p),
        12 => logic::state_generator(m, p),
        13 => logic::merge(m, p),
        14 => signal::envelope(m, cx.period, p),
        15 => signal::table(m, p),
        16 => signal::delay_line(m, cx.period, p),
        17 => math::mux(m, p),
        18 => math::demux(m, p),
        19 => math::min(m, p),
        20 => math::max(m, p),
        21 => math::scale(m, p),
        22 => math::add(m, p),
        23 => math::subtract(m, p),
        24 => math::multiply(m, p),
        25 => math::divide(m, p),
        26 => math::modulo(m, p),
        27 => return player::player(cx, p),
        28 => signal::oscillator(m, cx.period, p),
        29 => signal::ramp(m, cx.period, p),
        30 => math::add_capped(m, p),
        31 => math::subtract_floored(m, p),
        32 => math::multiply_capped(m, p),
        33 => math::min2(m, p),
        34 => math::max2(m, p),
        35 => math::scale2(m, p),
        36 => math::add2(m, p),
        38 => return player::class_controller(cx, p),
        other => return Err(Error::UnknownFunction(other)),
    })
}
