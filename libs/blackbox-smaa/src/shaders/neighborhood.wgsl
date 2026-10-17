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
// Pass 3, neighbourhood blending: SMAANeighborhoodBlendingVS and SMAANeighborhoodBlendingPS. Blends each pixel
// with the neighbour its blend weights pick, using the bilinear filter to mix the two. Writes the output view.

@group(0) @binding(2) var color_tex: texture_2d<f32>;
@group(0) @binding(3) var blend_tex: texture_2d<f32>;

struct NeighborhoodVertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) offset: vec4<f32>,
    @location(1) texcoord: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> NeighborhoodVertexOut {
    let corner = fullscreen_corner(vertex_index);
    let texcoord = fullscreen_texcoord(corner);
    var o: NeighborhoodVertexOut;
    o.position = vec4<f32>(corner, 1.0, 1.0);
    o.texcoord = texcoord;
    o.offset = rt_offset(vec4<f32>(1.0, 0.0, 0.0, 1.0), texcoord);
    return o;
}

@fragment
fn fs_main(i: NeighborhoodVertexOut) -> @location(0) vec4<f32> {
    // Blend weights of the right, top, bottom and left neighbours.
    var a = vec4<f32>(0.0);
    a.x = textureSampleLevel(blend_tex, linear_sampler, i.offset.xy, 0.0).a;
    a.y = textureSampleLevel(blend_tex, linear_sampler, i.offset.zw, 0.0).g;
    let centre = textureSampleLevel(blend_tex, linear_sampler, i.texcoord, 0.0);
    a.w = centre.x;
    a.z = centre.z;

    // No blending weight: the pixel is kept as it is.
    if dot(a, vec4<f32>(1.0)) < 1e-5 {
        return textureSampleLevel(color_tex, linear_sampler, i.texcoord, 0.0);
    }

    // Horizontal or vertical neighbour, whichever carries the larger weight.
    let h = max(a.x, a.z) > max(a.y, a.w);
    var blending_offset = vec4<f32>(0.0, a.y, 0.0, a.w);
    var blending_weight = a.yw;
    if h {
        blending_offset = vec4<f32>(a.x, 0.0, a.z, 0.0);
        blending_weight = a.xz;
    }
    blending_weight = blending_weight / dot(blending_weight, vec2<f32>(1.0));

    let blending_coord = blending_offset * vec4<f32>(rt_metrics.xy, -rt_metrics.xy) + i.texcoord.xyxy;
    var color = blending_weight.x * textureSampleLevel(color_tex, linear_sampler, blending_coord.xy, 0.0);
    color += blending_weight.y * textureSampleLevel(color_tex, linear_sampler, blending_coord.zw, 0.0);
    return color;
}
