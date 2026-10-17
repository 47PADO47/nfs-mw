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
// Pass 1, luma edge detection: SMAAEdgeDetectionVS and SMAALumaEdgeDetectionPS (predication off).
// Writes the edges texture (RG8: left and top edges), cleared to zero by the caller.

@group(0) @binding(2) var color_tex: texture_2d<f32>;

struct EdgeVertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) offset0: vec4<f32>,
    @location(1) offset1: vec4<f32>,
    @location(2) offset2: vec4<f32>,
    @location(3) texcoord: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> EdgeVertexOut {
    let corner = fullscreen_corner(vertex_index);
    let texcoord = fullscreen_texcoord(corner);
    var o: EdgeVertexOut;
    o.position = vec4<f32>(corner, 1.0, 1.0);
    o.texcoord = texcoord;
    o.offset0 = rt_offset(vec4<f32>(-1.0, 0.0, 0.0, -1.0), texcoord);
    o.offset1 = rt_offset(vec4<f32>(1.0, 0.0, 0.0, 1.0), texcoord);
    o.offset2 = rt_offset(vec4<f32>(-2.0, 0.0, 0.0, -2.0), texcoord);
    return o;
}

fn luma_at(uv: vec2<f32>) -> f32 {
    return dot(textureSampleLevel(color_tex, linear_sampler, uv, 0.0).rgb, LUMA_WEIGHTS);
}

@fragment
fn fs_main(i: EdgeVertexOut) -> @location(0) vec2<f32> {
    let threshold = vec2<f32>(SMAA_THRESHOLD);
    let l = luma_at(i.texcoord);
    let l_left = luma_at(i.offset0.xy);
    let l_top = luma_at(i.offset0.zw);
    let delta_lt = abs(vec2<f32>(l) - vec2<f32>(l_left, l_top));
    var edges = step(threshold, delta_lt);
    // No edge on this pixel: leave the cleared value, as the reference's discard does.
    if dot(edges, vec2<f32>(1.0)) == 0.0 {
        discard;
    }

    let l_right = luma_at(i.offset1.xy);
    let l_bottom = luma_at(i.offset1.zw);
    let delta_rb = abs(vec2<f32>(l) - vec2<f32>(l_right, l_bottom));
    let max_direct = max(delta_lt, delta_rb);

    let l_leftleft = luma_at(i.offset2.xy);
    let l_toptop = luma_at(i.offset2.zw);
    let delta_outer = abs(vec2<f32>(l_left, l_top) - vec2<f32>(l_leftleft, l_toptop));
    let max_all = max(max_direct, delta_outer);
    let final_delta = max(max_all.x, max_all.y);

    // Local contrast adaptation.
    edges *= step(vec2<f32>(final_delta), SMAA_LOCAL_CONTRAST_ADAPTATION_FACTOR * delta_lt);
    return edges;
}
