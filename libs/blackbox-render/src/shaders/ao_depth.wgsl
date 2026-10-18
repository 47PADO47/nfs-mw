// Horizon search and analytic slice integral adapted from XeGTAO (MIT).
// Copyright (C) 2016-2021, Intel Corporation
// SPDX-License-Identifier: MIT
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
// Depth-aware linear-depth hierarchy for horizon taps. Based on the depth-prefiltering
// approach documented by XeGTAO (MIT); independent implementation, no temporal inputs.
@group(0) @binding(2) var previous_depth: texture_2d<f32>;
@group(0) @binding(3) var next_depth: texture_storage_2d<r32float, write>;
override FROM_SCENE: bool = true;

fn hierarchy_depth(pixel: vec2<i32>) -> f32 {
    if !FROM_SCENE { return textureLoad(previous_depth, pixel, 0).r; }
    let d = textureLoad(scene_depth, pixel, 0);
    if d <= 0.0 { return 0.0; }
    return params.projection.z / d;
}

fn hierarchy_weight(depth: f32, farthest: f32) -> f32 {
    if params.fog.z > 0.5 {
        // XeGTAO's radius-scaled linear falloff, from its FilterDepths core.
        let radius = params.effect.x * (0.75 * 1.457);
        return clamp((radius - (farthest-depth)) / (0.615 * radius), 0.0, 1.0);
    }
    let tolerance = max(params.effect.x * 0.75, 0.01);
    let weight = max(0.0, 1.0 - (farthest-depth)/tolerance);
    return weight * weight;
}

@compute @workgroup_size(8, 8)
fn reduce_depth(@builtin(global_invocation_id) id: vec3<u32>) {
    let output_size = textureDimensions(next_depth);
    if any(id.xy >= output_size) { return; }
    var input_size = textureDimensions(previous_depth);
    if FROM_SCENE { input_size = textureDimensions(scene_depth); }
    // Area coverage includes the final odd row/column, rather than discarding it.
    let scale = vec2<f32>(input_size) / vec2<f32>(output_size);
    let begin = vec2<i32>(floor(vec2<f32>(id.xy) * scale));
    let end = vec2<i32>(floor(vec2<f32>(id.xy + vec2<u32>(1)) * scale));
    var farthest = 0.0;
    for (var y = begin.y; y < end.y; y++) {
        for (var x = begin.x; x < end.x; x++) {
            farthest = max(farthest, hierarchy_depth(vec2<i32>(x,y)));
        }
    }
    if farthest <= 0.0 {
        textureStore(next_depth, id.xy, vec4<f32>(0.0));
        return;
    }
    var total = 0.0;
    var weights = 0.0;
    for (var y = begin.y; y < end.y; y++) {
        for (var x = begin.x; x < end.x; x++) {
            let d = hierarchy_depth(vec2<i32>(x,y));
            if d <= 0.0 { continue; }
            let weight = hierarchy_weight(d,farthest);
            total += d * weight;
            weights += weight;
        }
    }
    textureStore(next_depth, id.xy, vec4<f32>(total / max(weights, 1e-8)));
}
