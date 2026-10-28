//! The four-lane ramp block of a corona and of each of its layers. Format: `docs/formats/chameleon-corona.md`.
//!
//! The executable stores the curves of a corona as eight rows of four floats (a 0x90 byte block, its first 16 bytes the
//! object head) and evaluates them with SSE, one lane at a time and all four alike. Each lane is a two-piece ramp: below
//! a threshold one line, above it another, clamped to 1 from above, then scaled and offset. The rows are named here by
//! their offset in the block, as the evaluator reads them.

use crate::f32_at;

/// Where the block starts in a corona object or in one of its layers.
pub const BLOCK_AT: usize = 0x10;
/// The rows of four floats, from the first float after the 16-byte head.
const ROWS: usize = 8;
const ROWS_AT: usize = 0x10;
/// Lanes in a row.
pub const LANES: usize = 4;

/// Eight rows of four floats read from a block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveBlock {
    /// Rows at block +0x10 to +0x80: intercept below the threshold, intercept above, slope below, slope above,
    /// threshold, gain below, offset, gain above.
    pub rows: [[f32; LANES]; ROWS],
}

impl CurveBlock {
    /// Read the block of the structure that starts at `at` of `main` (the block is 0x10 bytes into it).
    pub fn parse(main: &[u8], at: usize) -> Option<Self> {
        let start = at + BLOCK_AT + ROWS_AT;
        let mut rows = [[0.0; LANES]; ROWS];
        for (r, row) in rows.iter_mut().enumerate() {
            for (lane, value) in row.iter_mut().enumerate() {
                *value = f32_at(main, start + 16 * r + 4 * lane)?;
            }
        }
        Some(Self { rows })
    }

    /// The four outputs for the four inputs: per lane `min(slope * x + intercept, 1) * gain + offset`, where the
    /// intercept, slope and gain are the "below" ones when `x` is under the lane's threshold and the "above" ones
    /// otherwise.
    pub fn eval(&self, x: [f32; LANES]) -> [f32; LANES] {
        let [low_c, high_c, low_s, high_s, threshold, low_g, offset, high_g] = self.rows;
        let mut out = [0.0; LANES];
        for lane in 0..LANES {
            let below = x[lane] < threshold[lane];
            let intercept = pick(below, low_c[lane], high_c[lane]);
            let slope = pick(below, low_s[lane], high_s[lane]);
            let gain = pick(below, low_g[lane], high_g[lane]);
            let ramp = (slope * x[lane] + intercept).min(1.0);
            out[lane] = ramp * gain + offset[lane];
        }
        out
    }

    /// One lane's output.
    pub fn lane(&self, lane: usize, x: f32) -> f32 {
        let mut input = [0.0; LANES];
        input[lane] = x;
        self.eval(input)[lane]
    }
}

fn pick(first: bool, a: f32, b: f32) -> f32 {
    if first {
        return a;
    }
    b
}
