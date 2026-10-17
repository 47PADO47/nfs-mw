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
// Pass 2, blending weights: SMAABlendingWeightCalculationVS and SMAABlendingWeightCalculationPS for SMAA 1x.
// Reads the edges (RG8) and writes the blend weights (RGBA8): horizontal weights in rg, vertical in ba.
// Uses the search helpers of blend_search.wgsl.

@group(0) @binding(2) var edges_tex: texture_2d<f32>;
@group(0) @binding(3) var area_tex: texture_2d<f32>;
@group(0) @binding(4) var search_tex: texture_2d<f32>;

struct BlendVertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) pixcoord: vec2<f32>,
    @location(1) offset0: vec4<f32>,
    @location(2) offset1: vec4<f32>,
    @location(3) offset2: vec4<f32>,
    @location(4) texcoord: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> BlendVertexOut {
    let corner = fullscreen_corner(vertex_index);
    let texcoord = fullscreen_texcoord(corner);
    var o: BlendVertexOut;
    o.position = vec4<f32>(corner, 1.0, 1.0);
    o.texcoord = texcoord;
    o.pixcoord = texcoord * rt_metrics.zw;
    // Search helpers: the offsets point at the pixel's neighbours (the reference's @PSEUDO_GATHER4 taps).
    o.offset0 = rt_offset(vec4<f32>(-0.25, -0.125, 1.25, -0.125), texcoord);
    o.offset1 = rt_offset(vec4<f32>(-0.125, -0.25, -0.125, 1.25), texcoord);
    // The ends of the search loops: mad(RT.xxyy, (-2, 2, -2, 2) * MAX_SEARCH_STEPS, (offset0.xz, offset1.yw)).
    o.offset2 = vec4<f32>(rt_metrics.x, rt_metrics.x, rt_metrics.y, rt_metrics.y)
        * vec4<f32>(-2.0, 2.0, -2.0, 2.0) * SMAA_MAX_SEARCH_STEPS
        + vec4<f32>(o.offset0.xz, o.offset1.yw);
    return o;
}

// The horizontal half of SMAABlendingWeightCalculationPS: search left and right along the row.
fn horizontal_weights(
    pixcoord: vec2<f32>,
    texcoord: vec2<f32>,
    offset0: vec4<f32>,
    offset1: vec4<f32>,
    offset2: vec4<f32>,
) -> vec2<f32> {
    var coords = vec3<f32>(0.0);
    coords.x = search_x_left(edges_tex, search_tex, offset0.xy, offset2.x);
    // Row of the crossing-edge fetches, a quarter texel above the centre (@CROSSING_OFFSET).
    coords.y = offset1.y;
    var d = vec2<f32>(coords.x, 0.0);
    // Left crossing edge, sampled at the -0.25 row to tell the two edge values apart.
    let e1 = textureSampleLevel(edges_tex, linear_sampler, coords.xy, 0.0).r;

    coords.z = search_x_right(edges_tex, search_tex, offset0.zw, offset2.y);
    d.y = coords.z;

    // Distances in pixels.
    d = abs(round(rt_metrics.z * d - vec2<f32>(pixcoord.x)));
    let sqrt_d = sqrt(d);

    // Right crossing edge.
    let e2 = textureSampleLevel(edges_tex, linear_sampler, coords.zy, 0.0, vec2<i32>(1, 0)).r;

    var weights = area_ortho(area_tex, sqrt_d, e1, e2, 0.0);

    // Fix the corners.
    coords.y = texcoord.y;
    weights = detect_horizontal_corner(edges_tex, weights, vec4<f32>(coords.x, coords.y, coords.z, coords.y), d);
    return weights;
}

// The vertical half of SMAABlendingWeightCalculationPS: search up and down the column.
fn vertical_weights(
    pixcoord: vec2<f32>,
    texcoord: vec2<f32>,
    offset0: vec4<f32>,
    offset1: vec4<f32>,
    offset2: vec4<f32>,
) -> vec2<f32> {
    var coords = vec3<f32>(0.0);
    coords.y = search_y_up(edges_tex, search_tex, offset1.xy, offset2.z);
    coords.x = offset0.x;
    var d = vec2<f32>(coords.y, 0.0);

    // Top crossing edge.
    let e1 = textureSampleLevel(edges_tex, linear_sampler, coords.xy, 0.0).g;

    coords.z = search_y_down(edges_tex, search_tex, offset1.zw, offset2.w);
    d.y = coords.z;

    d = abs(round(rt_metrics.w * d - vec2<f32>(pixcoord.y)));
    let sqrt_d = sqrt(d);

    // Bottom crossing edge.
    let e2 = textureSampleLevel(edges_tex, linear_sampler, coords.xz, 0.0, vec2<i32>(0, 1)).g;

    var weights = area_ortho(area_tex, sqrt_d, e1, e2, 0.0);

    coords.x = texcoord.x;
    weights = detect_vertical_corner(edges_tex, weights, vec4<f32>(coords.x, coords.y, coords.x, coords.z), d);
    return weights;
}

@fragment
fn fs_main(i: BlendVertexOut) -> @location(0) vec4<f32> {
    var weights = vec4<f32>(0.0);
    var e = textureSampleLevel(edges_tex, linear_sampler, i.texcoord, 0.0).rg;

    // Edge at north: diagonals have both north and west edges, so searching one boundary is enough.
    if e.y > 0.0 {
        let diag = calculate_diag_weights(edges_tex, area_tex, i.texcoord, e);
        weights.x = diag.x;
        weights.y = diag.y;
        // Diagonals take priority: a found diagonal skips the horizontal search and the vertical one.
        let no_diag = diag.x == -diag.y;
        if no_diag {
            let h = horizontal_weights(i.pixcoord, i.texcoord, i.offset0, i.offset1, i.offset2);
            weights.x = h.x;
            weights.y = h.y;
        }
        e.x = select(0.0, e.x, no_diag);
    }

    // Edge at west.
    if e.x > 0.0 {
        let v = vertical_weights(i.pixcoord, i.texcoord, i.offset0, i.offset1, i.offset2);
        weights.z = v.x;
        weights.w = v.y;
    }
    return weights;
}
