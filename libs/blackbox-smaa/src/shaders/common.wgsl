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
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software. As clarification, there is no
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
// Shared part of the SMAA 1x passes: a WGSL port of SMAA.hlsl (reference by Jimenez et al.) at preset HIGH,
// the way the smaa crate 0.21.0 builds it through its GLSL-4 porting layer. Every sample is linear-filtered
// (the GLSL layer maps even the "point" fetches to the linear sampler), diagonal and corner detection are on,
// predication and reprojection are off, and the subsample indices are all zero (SMAA 1x).
// Each pass is one module: this file, then the pass's own file (see src/shaders.rs).

// SMAA_PRESET_HIGH.
const SMAA_THRESHOLD: f32 = 0.1;
const SMAA_MAX_SEARCH_STEPS: f32 = 16.0;
const SMAA_MAX_SEARCH_STEPS_DIAG: f32 = 8.0;
const SMAA_CORNER_ROUNDING_NORM: f32 = 0.25;

// Reference defaults and fixed tables.
const SMAA_LOCAL_CONTRAST_ADAPTATION_FACTOR: f32 = 2.0;
const SMAA_AREATEX_MAX_DISTANCE: f32 = 16.0;
const SMAA_AREATEX_MAX_DISTANCE_DIAG: f32 = 20.0;
// The area table is 160x560 (src/area.rs), the search table 64x16 packed (src/search.rs).
const SMAA_AREATEX_PIXEL_SIZE: vec2<f32> = vec2<f32>(1.0 / 160.0, 1.0 / 560.0);
const SMAA_AREATEX_SUBTEX_SIZE: f32 = 1.0 / 7.0;
const SMAA_SEARCHTEX_SIZE: vec2<f32> = vec2<f32>(66.0, 33.0);
const SMAA_SEARCHTEX_PACKED_SIZE: vec2<f32> = vec2<f32>(64.0, 16.0);
const SMAA_SEARCHTEX_PACKED_INV: vec2<f32> = vec2<f32>(1.0 / 64.0, 1.0 / 16.0);
const LUMA_WEIGHTS: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

// The sampler is linear with clamp-to-edge; `rt_metrics` is (1/width, 1/height, width, height).
@group(0) @binding(0) var linear_sampler: sampler;
@group(0) @binding(1) var<uniform> rt_metrics: vec4<f32>;

// The oversized triangle of the reference (vertex 0 at (-1, -1), 1 at (-1, 3), 2 at (3, -1)).
fn fullscreen_corner(vertex_index: u32) -> vec2<f32> {
    var corners = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(-1.0, 3.0), vec2<f32>(3.0, -1.0));
    return corners[vertex_index];
}

fn fullscreen_texcoord(corner: vec2<f32>) -> vec2<f32> {
    return corner * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
}

// Reference `mad(SMAA_RT_METRICS.xyxy, dir, texcoord.xyxy)`: neighbour offsets in texel steps.
fn rt_offset(dir: vec4<f32>, texcoord: vec2<f32>) -> vec4<f32> {
    return rt_metrics.xyxy * dir + texcoord.xyxy;
}
