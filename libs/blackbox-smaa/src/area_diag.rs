// Ported from Scripts/AreaTex.py of SMAA (areadiag and helpers).
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
//! Diagonal areas for SMAA's `AreaTex`, computed by brute-force sampling.
//!
//! Ported from `areadiag` and its helpers in `Scripts/AreaTex.py` in
//! <https://github.com/iryoku/smaa>. Copyright (C) 2013 Jorge Jimenez,
//! Jose I. Echevarria, Belen Masia, Fernando Navarro, Diego Gutierrez.
//! Licensed under the MIT licence, see `LICENSE-SMAA`.
//!
//! The arithmetic keeps the script's operation order so that the f64 results,
//! and therefore the quantised bytes, match the reference texture exactly.

use crate::area::{Pt, add, average, select};

/// (x, y) slot index of each of the 16 diagonal patterns, in units of the slot size.
pub(crate) const EDGES_DIAG: [(usize, usize); 16] = [
    (0, 0),
    (1, 0),
    (0, 2),
    (1, 2),
    (2, 0),
    (3, 0),
    (2, 2),
    (3, 2),
    (0, 1),
    (1, 1),
    (0, 3),
    (1, 3),
    (2, 1),
    (3, 1),
    (2, 3),
    (3, 3),
];

/// Samples per axis used to estimate the covered fraction of a pixel.
const SAMPLES_DIAG: usize = 30;

/// Area pair `(left, right)` of diagonal pattern `pattern` at distances
/// `left` and `right`, for the subsample `offset`.
pub(crate) fn area_diag(pattern: usize, left: f64, right: f64, offset: Pt) -> Pt {
    let d = left + right + 1.0;
    // `vec2(a, b) + vec2(d, d)` in the script.
    let shifted = |a: f64, b: f64| (a + d, b + d);
    match pattern {
        0 => average(
            line_area(pattern, (1.0, 1.0), shifted(1.0, 1.0), left, offset),
            line_area(pattern, (1.0, 0.0), shifted(1.0, 0.0), left, offset),
        ),
        1 => average(
            line_area(pattern, (1.0, 0.0), shifted(0.0, 0.0), left, offset),
            line_area(pattern, (1.0, 0.0), shifted(1.0, 0.0), left, offset),
        ),
        2 => average(
            line_area(pattern, (0.0, 0.0), shifted(1.0, 0.0), left, offset),
            line_area(pattern, (1.0, 0.0), shifted(1.0, 0.0), left, offset),
        ),
        3 => line_area(pattern, (1.0, 0.0), shifted(1.0, 0.0), left, offset),
        4 => average(
            line_area(pattern, (1.0, 1.0), shifted(0.0, 0.0), left, offset),
            line_area(pattern, (1.0, 1.0), shifted(1.0, 0.0), left, offset),
        ),
        5 => average(
            line_area(pattern, (1.0, 1.0), shifted(0.0, 0.0), left, offset),
            line_area(pattern, (1.0, 0.0), shifted(1.0, 0.0), left, offset),
        ),
        6 => line_area(pattern, (1.0, 1.0), shifted(1.0, 0.0), left, offset),
        7 => average(
            line_area(pattern, (1.0, 1.0), shifted(1.0, 0.0), left, offset),
            line_area(pattern, (1.0, 0.0), shifted(1.0, 0.0), left, offset),
        ),
        8 => average(
            line_area(pattern, (0.0, 0.0), shifted(1.0, 1.0), left, offset),
            line_area(pattern, (1.0, 0.0), shifted(1.0, 1.0), left, offset),
        ),
        9 => line_area(pattern, (1.0, 0.0), shifted(1.0, 1.0), left, offset),
        10 => average(
            line_area(pattern, (0.0, 0.0), shifted(1.0, 1.0), left, offset),
            line_area(pattern, (1.0, 0.0), shifted(1.0, 0.0), left, offset),
        ),
        11 => average(
            line_area(pattern, (1.0, 0.0), shifted(1.0, 1.0), left, offset),
            line_area(pattern, (1.0, 0.0), shifted(1.0, 0.0), left, offset),
        ),
        12 => line_area(pattern, (1.0, 1.0), shifted(1.0, 1.0), left, offset),
        13 => average(
            line_area(pattern, (1.0, 1.0), shifted(1.0, 1.0), left, offset),
            line_area(pattern, (1.0, 0.0), shifted(1.0, 1.0), left, offset),
        ),
        14 => average(
            line_area(pattern, (1.0, 1.0), shifted(1.0, 1.0), left, offset),
            line_area(pattern, (1.0, 1.0), shifted(1.0, 0.0), left, offset),
        ),
        15 => average(
            line_area(pattern, (1.0, 1.0), shifted(1.0, 1.0), left, offset),
            line_area(pattern, (1.0, 0.0), shifted(1.0, 0.0), left, offset),
        ),
        _ => unreachable!("diagonal pattern index must be below 16, got {pattern}"),
    }
}

/// Area under the line `p1 -> p2` for the pattern's crossing edges, with the
/// subsample `offset` applied to the endpoints that touch a crossing edge.
fn line_area(pattern: usize, p1: Pt, p2: Pt, left: f64, offset: Pt) -> Pt {
    let (e1, e2) = EDGES_DIAG[pattern];
    let p1 = select(e1 > 0, add(p1, offset), p1);
    let p2 = select(e2 > 0, add(p2, offset), p2);
    let a1 = sampled_fraction(p1, p2, (1.0 + left, 0.0 + left));
    let a2 = sampled_fraction(p1, p2, (1.0 + left, 1.0 + left));
    (1.0 - a1, a2)
}

/// Fraction of the pixel at `p` (sampled on a grid of `[0, 1]^2`) lying on the
/// positive side of the line `p1 -> p2`.
fn sampled_fraction(p1: Pt, p2: Pt, p: Pt) -> f64 {
    let mut count = 0usize;
    for x in 0..SAMPLES_DIAG {
        for y in 0..SAMPLES_DIAG {
            let step = (SAMPLES_DIAG - 1) as f64;
            let o = (x as f64 / step, y as f64 / step);
            if inside(p1, p2, add(p, o)) {
                count += 1;
            }
        }
    }
    count as f64 / (SAMPLES_DIAG * SAMPLES_DIAG) as f64
}

/// Whether sample point `q` lies on the positive side of the line `p1 -> p2`,
/// measured from the segment midpoint. Degenerate segments count as inside.
fn inside(p1: Pt, p2: Pt, q: Pt) -> bool {
    if p1 == p2 {
        return true;
    }
    let xm = (p1.0 + p2.0) / 2.0;
    let ym = (p1.1 + p2.1) / 2.0;
    let a = p2.1 - p1.1;
    let b = p1.0 - p2.0;
    let c = a * (q.0 - xm) + b * (q.1 - ym);
    c > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::area::to_byte;

    const ZERO_OFFSET: Pt = (0.0, 0.0);

    #[test]
    fn pattern_3_at_origin_matches_reference_texel() {
        // Reference AreaTex texel (100, 40) is [129, 0]: pattern 3, distances (0, 0).
        let areas = area_diag(3, 0.0, 0.0, ZERO_OFFSET);
        assert_eq!(to_byte(areas.0), 129);
        assert_eq!(to_byte(areas.1), 0);
    }

    #[test]
    fn degenerate_segment_is_inside_everywhere() {
        assert!(inside((1.0, 1.0), (1.0, 1.0), (0.3, 0.7)));
    }

    #[test]
    fn sampled_fraction_is_full_for_degenerate_line() {
        let full = sampled_fraction((2.0, 2.0), (2.0, 2.0), (0.0, 0.0));
        assert_eq!(full, 1.0);
    }

    #[test]
    fn areas_stay_in_unit_range() {
        for pattern in 0..16 {
            for left in 0..20 {
                for right in 0..20 {
                    let (a, b) = area_diag(pattern, left as f64, right as f64, ZERO_OFFSET);
                    assert!((0.0..=1.0).contains(&a), "p{pattern} {left} {right}: {a}");
                    assert!((0.0..=1.0).contains(&b), "p{pattern} {left} {right}: {b}");
                }
            }
        }
    }
}
