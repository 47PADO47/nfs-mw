// Ported from Scripts/AreaTex.py of SMAA.
// Copyright (C) 2013 Jorge Jimenez (jorge@iryoku.com)
// Copyright (C) 2013 Jose I. Echevarria (joseignacioechevarria@gmail.com)
// Copyright (C) 2013 Belen Masia (bmasia@unizar.es)
// Copyright (C) 2013 Fernando Navarro (fernandn@microsoft.com)
// Copyright (C) 2013 Diego Gutierrez (diegog@unizar.es)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software. As clarification, there is no
// requirement that the copyright notice and permission be included in binary
// distributions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
// FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
// IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// Ported from SMAA (https://github.com/iryoku/smaa). Upstream notice is also
// kept in LICENSE-SMAA.
//
//! SMAA's `AreaTex` lookup texture, generated in code.
//!
//! Ported from `Scripts/AreaTex.py` in <https://github.com/iryoku/smaa>.
//! Copyright (C) 2013 Jorge Jimenez, Jose I. Echevarria, Belen Masia,
//! Fernando Navarro, Diego Gutierrez. Licensed under the MIT licence, see
//! `LICENSE-SMAA`.
//!
//! Layout of the 160 x 560 texture, RG8, row-major, 2 bytes per texel
//! (`R` = area left of the edge, `G` = area right of it):
//!
//! - `x < 80`: orthogonal patterns. Seven subsample sets, each 80 rows tall.
//! - `x >= 80`: diagonal patterns. Five subsample sets, each 80 rows tall.
//! - Pattern `p` occupies a `16 x 16` (orthogonal) or `20 x 20` (diagonal)
//!   block at its slot in the 4x4 grid given by `EDGES_ORTHO` / `EDGES_DIAG`.
//!
//! SMAA 1x only samples subsample set 0 (rows `0..80`), but the whole texture
//! is generated so that the offsets used by the shader line up.

use std::sync::OnceLock;

use crate::area_diag::{EDGES_DIAG, area_diag};

/// Texture size in texels, `(width, height)`.
pub const AREA_SIZE: (u32, u32) = (160, 560);

/// An `(f64, f64)` pair of areas, or of coordinates in the script's `vec2`.
pub(crate) type Pt = (f64, f64);

/// Maximum distance covered by an orthogonal slot (`SIZE_ORTHO` in the script).
const ORTHO_DISTANCE: usize = 16;
/// Maximum distance covered by a diagonal slot (`SIZE_DIAG` in the script).
const DIAG_DISTANCE: usize = 20;
/// Rows used by one subsample set.
const SET_ROWS: usize = 80;
/// First column of the diagonal half.
const DIAG_X: usize = 80;
/// Maximum distance over which u-shapes are smoothed (`SMOOTH_MAX_DISTANCE`).
const SMOOTH_MAX_DISTANCE: f64 = 32.0;

/// Subsample offsets of the orthogonal sets, in set order.
const ORTHO_OFFSETS: [f64; 7] = [0.0, -0.25, 0.25, -0.125, 0.125, -0.375, 0.375];
/// Subsample offsets of the diagonal sets, in set order.
const DIAG_OFFSETS: [Pt; 5] = [(0.0, 0.0), (0.25, -0.25), (-0.25, 0.25), (0.125, -0.125), (-0.125, 0.125)];

/// (x, y) slot index of each of the 16 orthogonal patterns, in units of the slot size.
const EDGES_ORTHO: [(usize, usize); 16] = [
    (0, 0),
    (3, 0),
    (0, 3),
    (3, 3),
    (1, 0),
    (4, 0),
    (1, 3),
    (4, 3),
    (0, 1),
    (3, 1),
    (0, 4),
    (3, 4),
    (1, 1),
    (4, 1),
    (1, 4),
    (4, 4),
];

static AREA_CACHE: OnceLock<Vec<u8>> = OnceLock::new();

/// Generates the full `AreaTex` as RG8 bytes, row-major, `160 * 560 * 2` long.
///
/// Deterministic; the result is bit-identical to the texture shipped in the
/// `smaa` crate. Use [`area_texture_cached`] to avoid recomputing it.
pub fn area_texture() -> Vec<u8> {
    let (width, height) = AREA_SIZE;
    let mut data = vec![0u8; width as usize * height as usize * 2];
    fill_orthogonal(&mut data);
    fill_diagonal(&mut data);
    data
}

/// Shared, lazily generated copy of [`area_texture`].
pub fn area_texture_cached() -> &'static [u8] {
    AREA_CACHE.get_or_init(area_texture).as_slice()
}

/// RG8 texel `[left, right]` at `(x, y)` of the cached `AreaTex`.
///
/// Debugging helper for the tests. Panics when `(x, y)` is outside [`AREA_SIZE`].
#[cfg(test)]
pub fn area_texel(x: u32, y: u32) -> [u8; 2] {
    let (width, height) = AREA_SIZE;
    assert!(x < width && y < height, "area texel ({x}, {y}) is outside {width}x{height}");
    let index = (y as usize * width as usize + x as usize) * 2;
    let data = area_texture_cached();
    [data[index], data[index + 1]]
}

fn fill_orthogonal(data: &mut [u8]) {
    for (set, &offset) in ORTHO_OFFSETS.iter().enumerate() {
        for (pattern, &(slot_x, slot_y)) in EDGES_ORTHO.iter().enumerate() {
            for left in 0..ORTHO_DISTANCE {
                for right in 0..ORTHO_DISTANCE {
                    // The script looks the subtexture up at squared distances.
                    let areas = ortho_area(pattern, squared(left), squared(right), offset);
                    let x = ORTHO_DISTANCE * slot_x + left;
                    let y = set * SET_ROWS + ORTHO_DISTANCE * slot_y + right;
                    place(data, x, y, areas);
                }
            }
        }
    }
}

fn fill_diagonal(data: &mut [u8]) {
    for (set, &offset) in DIAG_OFFSETS.iter().enumerate() {
        for (pattern, &(slot_x, slot_y)) in EDGES_DIAG.iter().enumerate() {
            for left in 0..DIAG_DISTANCE {
                for right in 0..DIAG_DISTANCE {
                    let areas = area_diag(pattern, left as f64, right as f64, offset);
                    let x = DIAG_X + DIAG_DISTANCE * slot_x + left;
                    let y = set * SET_ROWS + DIAG_DISTANCE * slot_y + right;
                    place(data, x, y, areas);
                }
            }
        }
    }
}

fn squared(distance: usize) -> f64 {
    (distance * distance) as f64
}

/// Writes one texel. Quantisation matches the script's `bytes()` helper.
fn place(data: &mut [u8], x: usize, y: usize, areas: Pt) {
    let index = (y * AREA_SIZE.0 as usize + x) * 2;
    data[index] = to_byte(areas.0);
    data[index + 1] = to_byte(areas.1);
}

/// `int(255.0 * a)` in the script. The `as` cast truncates like `int()` and
/// saturates to the byte range, which is what the script's pixel store does.
pub(crate) fn to_byte(area: f64) -> u8 {
    (255.0 * area) as u8
}

/// Picks `yes` when `cond` holds, otherwise `no`.
pub(crate) fn select<T>(cond: bool, yes: T, no: T) -> T {
    match cond {
        true => yes,
        false => no,
    }
}

/// Componentwise sum.
pub(crate) fn add(a: Pt, b: Pt) -> Pt {
    (a.0 + b.0, a.1 + b.1)
}

/// Componentwise `(a + b) / 2`.
pub(crate) fn average(a: Pt, b: Pt) -> Pt {
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
}

/// Returns `(0, 0)` unless `cond` holds. `area` is only evaluated when it does.
fn gated(cond: bool, area: impl FnOnce() -> Pt) -> Pt {
    if !cond {
        return (0.0, 0.0);
    }
    area()
}

/// Uses `blended` when the subsample offset is non-zero, else `plain`.
fn offset_blend(offset: f64, plain: impl FnOnce() -> Pt, blended: impl FnOnce() -> Pt) -> Pt {
    if offset.abs() > 0.0 {
        return blended();
    }
    plain()
}

/// Orthogonal area pair for `pattern` at distances `left` and `right`
/// (`areaortho` in the script).
fn ortho_area(pattern: usize, left: f64, right: f64, offset: f64) -> Pt {
    let d = left + right + 1.0;
    let o1 = 0.5 + offset;
    let o2 = 0.5 + offset - 1.0;
    let mid = (d / 2.0, 0.0);
    match pattern {
        1 => gated(left <= right, || segment_area((0.0, o2), mid, left)),
        2 => gated(left >= right, || segment_area(mid, (d, o2), left)),
        3 => {
            let a1 = segment_area((0.0, o2), mid, left);
            let a2 = segment_area(mid, (d, o2), left);
            let (a1, a2) = smooth_area(d, a1, a2);
            add(a1, a2)
        }
        4 => gated(left <= right, || segment_area((0.0, o1), mid, left)),
        6 => offset_blend(
            offset,
            || segment_area((0.0, o1), (d, o2), left),
            || {
                let whole = segment_area((0.0, o1), (d, o2), left);
                let halves = add(segment_area((0.0, o1), mid, left), segment_area(mid, (d, o2), left));
                average(whole, halves)
            },
        ),
        7 => segment_area((0.0, o1), (d, o2), left),
        8 => gated(left >= right, || segment_area(mid, (d, o1), left)),
        9 => offset_blend(
            offset,
            || segment_area((0.0, o2), (d, o1), left),
            || {
                let whole = segment_area((0.0, o2), (d, o1), left);
                let halves = add(segment_area((0.0, o2), mid, left), segment_area(mid, (d, o1), left));
                average(whole, halves)
            },
        ),
        11 => segment_area((0.0, o2), (d, o1), left),
        12 => {
            let a1 = segment_area((0.0, o1), mid, left);
            let a2 = segment_area(mid, (d, o1), left);
            let (a1, a2) = smooth_area(d, a1, a2);
            add(a1, a2)
        }
        13 => segment_area((0.0, o2), (d, o1), left),
        14 => segment_area((0.0, o1), (d, o2), left),
        // Patterns 0, 5, 10 and 15 have no area.
        _ => (0.0, 0.0),
    }
}

/// Area under the line `p1 -> p2` over the pixel column `[x, x + 1)`, split
/// into the parts left and right of the edge (`area` in `areaortho`).
fn segment_area(p1: Pt, p2: Pt, x: f64) -> Pt {
    let d = (p2.0 - p1.0, p2.1 - p1.1);
    let x1 = x;
    let x2 = x + 1.0;
    let y1 = p1.1 + d.1 * (x1 - p1.0) / d.0;
    let y2 = p1.1 + d.1 * (x2 - p1.0) / d.0;
    let inside = (x1 >= p1.0 && x1 < p2.0) || (x2 > p1.0 && x2 <= p2.0);
    if !inside {
        return (0.0, 0.0);
    }
    let same_side = 1.0_f64.copysign(y1) == 1.0_f64.copysign(y2);
    let trapezoid = same_side || y1.abs() < 1e-4 || y2.abs() < 1e-4;
    if trapezoid {
        return trapezoid_area(y1, y2);
    }
    triangle_area(p1, p2, d, y1, y2)
}

/// Area of a trapezoid spanning `y1..y2`, returned as `(left, right)`.
fn trapezoid_area(y1: f64, y2: f64) -> Pt {
    let a = (y1 + y2) / 2.0;
    if a < 0.0 {
        return (a.abs(), 0.0);
    }
    (0.0, a.abs())
}

/// Area of a segment crossing the pixel column into two triangles.
fn triangle_area(p1: Pt, p2: Pt, d: Pt, y1: f64, y2: f64) -> Pt {
    let x = -p1.1 * d.0 / d.1 + p1.0;
    let frac = x.fract();
    let a1 = select(x > p1.0, y1 * frac / 2.0, 0.0);
    let a2 = select(x < p2.0, y2 * (1.0 - frac) / 2.0, 0.0);
    let a = select(a1.abs() > a2.abs(), a1, -a2);
    if a < 0.0 {
        return (a1.abs(), a2.abs());
    }
    (a2.abs(), a1.abs())
}

/// Smooths small u-shapes by blending the two halves towards their square
/// roots over `SMOOTH_MAX_DISTANCE` (`smootharea` in the script).
fn smooth_area(d: f64, a1: Pt, a2: Pt) -> (Pt, Pt) {
    let b1 = sqrt_pair(a1);
    let b2 = sqrt_pair(a2);
    let p = (d / SMOOTH_MAX_DISTANCE).clamp(0.0, 1.0);
    (lerp(b1, a1, p), lerp(b2, a2, p))
}

/// `(sqrt(2a) * 0.5)` componentwise.
fn sqrt_pair(a: Pt) -> Pt {
    ((a.0 * 2.0).sqrt() * 0.5, (a.1 * 2.0).sqrt() * 0.5)
}

/// `a + (b - a) * p` componentwise.
fn lerp(a: Pt, b: Pt, p: f64) -> Pt {
    (a.0 + (b.0 - a.0) * p, a.1 + (b.1 - a.1) * p)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FNV-1a (64-bit) over `bytes`.
    fn fnv1a(bytes: &[u8]) -> u64 {
        bytes
            .iter()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, &byte| (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3))
    }

    /// FNV-1a of the reference `AREATEX_BYTES` shipped in smaa 0.21.0
    /// (`third_party/smaa/Textures/AreaTex.rs`).
    const REFERENCE_FNV1A: u64 = 0x247a_58bb_ba65_292d;

    #[test]
    fn size_is_160_by_560_rg8() {
        assert_eq!(AREA_SIZE, (160, 560));
        assert_eq!(area_texture_cached().len(), 160 * 560 * 2);
    }

    #[test]
    fn matches_reference_texture() {
        assert_eq!(fnv1a(area_texture_cached()), REFERENCE_FNV1A);
    }

    #[test]
    fn empty_patterns_have_no_area() {
        // Pattern 0 sits at slot (0, 0) and pattern 15 at slot (4, 4).
        for set in 0..7u32 {
            assert_eq!(area_texel(3, 80 * set + 3), [0, 0]);
        }
        assert_eq!(area_texel(67, 67), [0, 0]);
    }

    #[test]
    fn l_pattern_at_origin_has_known_area() {
        // Pattern 1 at slot (3, 0), distances (0, 0): the script gives 0.125,
        // stored as int(255 * 0.125) = 31 on the left channel.
        assert_eq!(area_texel(48, 0), [31, 0]);
    }

    #[test]
    fn unused_rows_and_diagonal_rows_are_zero() {
        // Diagonal sets only cover rows 0..400.
        assert_eq!(area_texel(120, 450), [0, 0]);
        assert_eq!(area_texel(159, 559), [0, 0]);
    }

    #[test]
    #[should_panic(expected = "outside")]
    fn texel_outside_texture_panics() {
        let _ = area_texel(160, 0);
    }
}
