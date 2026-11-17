//! The compute dispatches of one light's screen-space shadow: wavefronts laid out around the light's place on
//! screen, so that every thread of a wavefront walks one ray towards the light.
//!
//! Ported to Rust from `BuildDispatchList` in Bend Studio's `bend_sss_cpu.h` (2023 release); changed: Rust types,
//! a growable list in place of the fixed array, no expanded depth range.
//
// Copyright 2023 Sony Interactive Entertainment.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// This file has been modified from the original.

/// Threads in a wavefront; the compute shader's workgroup width.
pub(super) const WAVE: i32 = 64;

/// One dispatch of the shadow shader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Dispatch {
    /// Workgroup counts of the dispatch.
    pub waves: [u32; 3],
    /// The dispatch's wave offset in pixels, relative to the light.
    pub offset: [i32; 2],
}

/// The dispatches of one light, at most eight.
#[derive(Debug, Default, PartialEq)]
pub(super) struct DispatchList {
    /// The light for the shader: its pixel coordinate, its depth and the sign of its clip-space w.
    pub light: [f32; 4],
    pub dispatches: Vec<Dispatch>,
}

/// The dispatches that cover the inclusive pixel bounds `low..=high` of a `viewport` for a light at clip-space
/// `projection`: a direction with w zero through the view-projection matrix for a directional light, a position
/// with w one for a local light.
pub(super) fn dispatch_list(projection: [f32; 4], viewport: [i32; 2], low: [i32; 2], high: [i32; 2]) -> DispatchList {
    let mut result = DispatchList { light: [0.0; 4], dispatches: Vec::with_capacity(8) };
    // The shader's division loses precision for a light very far off screen, so the pixel coordinate is computed
    // with a w no smaller than this.
    let limit = 0.000002 * WAVE as f32;
    let mut w = projection[3];
    if (0.0..limit).contains(&w) {
        w = limit;
    }
    if w < 0.0 && w > -limit {
        w = -limit;
    }
    result.light[0] = (projection[0] / w * 0.5 + 0.5) * viewport[0] as f32;
    result.light[1] = (projection[1] / w * -0.5 + 0.5) * viewport[1] as f32;
    result.light[2] = match projection[3] == 0.0 {
        true => 0.0,
        false => projection[2] / projection[3],
    };
    result.light[3] = match projection[3] > 0.0 {
        true => 1.0,
        false => -1.0,
    };
    let light = [(result.light[0] + 0.5) as i32, (result.light[1] + 0.5) as i32];
    // The bounds relative to the light, y up.
    let biased = [
        low[0].saturating_sub(light[0]),
        light[1].saturating_sub(high[1]),
        high[0].saturating_sub(light[0]),
        light[1].saturating_sub(low[1]),
    ];
    // The four quadrants around the light: 0 bottom left, 1 bottom right, 2 top left, 3 top right. Each is a
    // rectangle with one corner on the light; one that is not square is split in two on its longer axis.
    for q in 0..4 {
        let vertical = q == 0 || q == 3;
        let (right, top) = (q & 1 != 0, q & 2 != 0);
        let side = |positive: bool, a: i32, b: i32| match positive {
            true => a,
            false => b.saturating_neg(),
        };
        let reach = |along: bool| match along {
            true => 2,
            false => 1,
        };
        let bounds = [
            side(right, biased[0], biased[2]).max(0) / WAVE,
            side(top, biased[1], biased[3]).max(0) / WAVE,
            side(right, biased[2], biased[0]).saturating_add(WAVE * reach(!vertical) - 1).max(0) / WAVE,
            side(top, biased[3], biased[1]).saturating_add(WAVE * reach(vertical) - 1).max(0) / WAVE,
        ];
        if bounds[2] - bounds[0] <= 0 || bounds[3] - bounds[1] <= 0 {
            continue;
        }
        let bias_x = i32::from(q == 2 || q == 3);
        let bias_y = i32::from(q == 1 || q == 3);
        let mut first = Dispatch {
            waves: [WAVE as u32, (bounds[2] - bounds[0]) as u32, (bounds[3] - bounds[1]) as u32],
            offset: [
                side(right, bounds[0], bounds[2]) + bias_x,
                match top {
                    true => -bounds[3],
                    false => bounds[1],
                } + bias_y,
            ],
        };
        // Where the diagonal ray from the light leaves the bounds, from the quadrant's far corner.
        let delta = match q {
            0 => biased[0].saturating_sub(biased[1]),
            1 => biased[2].saturating_add(biased[1]),
            2 => biased[0].saturating_neg().saturating_sub(biased[3]),
            _ => biased[2].saturating_neg().saturating_add(biased[3]),
        };
        let delta = delta.saturating_add(WAVE - 1) / WAVE;
        if delta <= 0 {
            result.dispatches.push(first);
            continue;
        }
        let mut second = first;
        let split = |count: u32| count.min(delta as u32);
        match q {
            0 => {
                // Split on y; the split is one larger on x.
                second.waves[2] = split(first.waves[2]);
                first.waves[2] -= second.waves[2];
                second.offset[1] = first.offset[1] + first.waves[2] as i32;
                second.offset[0] -= 1;
                second.waves[1] += 1;
            }
            1 => {
                // Split on x; the split is one larger on y.
                second.waves[1] = split(first.waves[1]);
                first.waves[1] -= second.waves[1];
                second.offset[0] = first.offset[0] + first.waves[1] as i32;
                second.waves[2] += 1;
            }
            2 => {
                // Split on x; the split is one larger on y.
                second.waves[1] = split(first.waves[1]);
                first.waves[1] -= second.waves[1];
                first.offset[0] += second.waves[1] as i32;
                second.waves[2] += 1;
                second.offset[1] -= 1;
            }
            _ => {
                // Split on y; the split is one larger on x.
                second.waves[2] = split(first.waves[2]);
                first.waves[2] -= second.waves[2];
                first.offset[1] += second.waves[2] as i32;
                second.waves[1] += 1;
            }
        }
        result.dispatches.extend([first, second].into_iter().filter(|d| d.waves[1] > 0 && d.waves[2] > 0));
    }
    // The shader expects the offsets in pixels.
    for dispatch in &mut result.dispatches {
        dispatch.offset = dispatch.offset.map(|offset| offset * WAVE);
    }
    result
}
