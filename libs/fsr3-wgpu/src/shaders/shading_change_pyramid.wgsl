// FSR 3.1 upscaler pass 3, shading change pyramid: for every pixel, the smallest relative difference
// between the sorted luma of the pixel's neighbourhood this frame and the motion-reprojected one of the
// previous frame, then a mean pyramid of that signed difference and of its sign. `cs_level0` builds the
// first (half-resolution) level from the frame, `cs_down` makes each following level from the last.
//
// Ported to WGSL from AMD's ffx_fsr3upscaler_shading_change_pyramid.h (FidelityFX SDK v1.1.4,
// FSR 3.1.4): https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK
//
// This file is part of the FidelityFX SDK.
//
// Copyright (C) 2024 Advanced Micro Devices, Inc.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files(the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and /or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions :
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
//
// Changes from the original: AMD's single-pass downsampler is replaced by separate dispatches, one
// per level, and only the three levels the shading change pass reads are built. The levels are
// separate textures. The mean of the 2x2 block repeats the edge texel where the block leaves the level.

@group(0) @binding(1) var dilated_motion_vectors: texture_2d<f32>;
@group(0) @binding(2) var current_luma: texture_2d<f32>;
@group(0) @binding(3) var previous_luma: texture_2d<f32>;
@group(0) @binding(4) var<storage, read> frame_info: array<vec4<f32>>;
@group(0) @binding(5) var out_level: texture_storage_2d<rg32float, write>;
@group(0) @binding(6) var source_level: texture_2d<f32>;

fn exposure() -> f32 {
    return frame_info[0].x;
}

// The cross-shaped neighbourhood compared between the frames.
fn sample_offset(index: i32) -> vec2<i32> {
    switch index {
        case 1: { return vec2<i32>(-1, 0); }
        case 2: { return vec2<i32>(1, 0); }
        case 3: { return vec2<i32>(0, -1); }
        case 4: { return vec2<i32>(0, 1); }
        default: { return vec2<i32>(0, 0); }
    }
}

fn compare_swap(values: ptr<function, array<f32, 5>>, i: i32, j: i32) {
    let low = min((*values)[i], (*values)[j]);
    (*values)[j] = max((*values)[i], (*values)[j]);
    (*values)[i] = low;
}

// A sorting network for five values.
fn sort_set(values: array<f32, 5>) -> array<f32, 5> {
    var s = values;
    compare_swap(&s, 0, 3);
    compare_swap(&s, 1, 4);
    compare_swap(&s, 0, 2);
    compare_swap(&s, 1, 3);
    compare_swap(&s, 0, 1);
    compare_swap(&s, 2, 4);
    compare_swap(&s, 1, 2);
    compare_swap(&s, 3, 4);
    compare_swap(&s, 2, 3);
    return s;
}

fn compute_minimum_difference(current: array<f32, 5>, previous: array<f32, 5>) -> f32 {
    let s0 = sort_set(current);
    let s1 = sort_set(previous);
    var min_diff = FP16_MAX - 1.0;
    var a = 0;
    var b = 0;
    if min(s0[4], s1[4]) > FP32_MIN {
        for (var i = 0; i < 5; i++) {
            if !(min_diff < FP16_MAX) {
                break;
            }
            let sa = s0[min(a, 4)];
            let sb = s1[min(b, 4)];
            var diff = sa - sb;
            if !(abs(diff) > FP16_MIN) {
                // Equal values end the search.
                min_diff = FP16_MAX;
                continue;
            }
            diff = sign(diff) * (1.0 - min_divided_by_max(sa, sb, 0.0));
            if abs(diff) < abs(min_diff) {
                min_diff = diff;
            }
            a += i32(sa < sb);
            b += i32(s0[min(a, 4)] >= sb);
        }
    }
    return min_diff * f32(min_diff < (FP16_MAX - 1.0));
}

fn current_luma_samples(uv: vec2<f32>) -> array<f32, 5> {
    let jittered = uv + cb.jitter / vec2<f32>(cb.render_size);
    let base = vec2<i32>(floor(jittered * vec2<f32>(cb.render_size)));
    var values: array<f32, 5>;
    for (var i = 0; i < SHADING_CHANGE_SET_SIZE; i++) {
        let pos = clamp_load(base, sample_offset(i), cb.render_size);
        values[i] = max(textureLoad(current_luma, pos, 0).x * exposure(), EPSILON);
    }
    return values;
}

fn previous_luma_samples(uv: vec2<f32>, motion_vector: vec2<f32>) -> array<f32, 5> {
    let jittered = uv + cb.previous_jitter / vec2<f32>(cb.previous_render_size);
    let base = vec2<i32>(floor((jittered + motion_vector) * vec2<f32>(cb.previous_render_size)));
    var values: array<f32, 5>;
    for (var i = 0; i < SHADING_CHANGE_SET_SIZE; i++) {
        let pos = clamp_load(base, sample_offset(i), cb.previous_render_size);
        values[i] = max(textureLoad(previous_luma, pos, 0).x * cb.delta_pre_exposure * exposure(), EPSILON);
    }
    return values;
}

// The signed relative luma change of one pixel, 0 when its reprojection leaves the screen.
fn compute_diff(pos: vec2<i32>) -> f32 {
    let motion_vector = textureLoad(dilated_motion_vectors, pos, 0).xy;
    let uv = (vec2<f32>(pos) + vec2<f32>(0.5)) / vec2<f32>(cb.render_size);
    let reprojected = uv + cb.previous_jitter / vec2<f32>(cb.previous_render_size) + motion_vector;
    if !is_uv_inside(reprojected) {
        return 0.0;
    }
    return compute_minimum_difference(current_luma_samples(uv), previous_luma_samples(uv, motion_vector));
}

@compute @workgroup_size(8, 8, 1)
fn cs_level0(@builtin(global_invocation_id) id: vec3<u32>) {
    let texel = vec2<i32>(id.xy);
    if any(texel >= vec2<i32>(textureDimensions(out_level))) {
        return;
    }
    // Each level 0 texel is the mean of 2x2 pixels: the difference and its sign.
    var sum = vec2<f32>(0.0);
    for (var i = 0; i < 4; i++) {
        let pos = clamp_load(texel * 2, tap_offset(i), cb.render_size);
        let diff = compute_diff(pos);
        sum += vec2<f32>(diff, select(0.0, sign(diff), diff != 0.0));
    }
    textureStore(out_level, texel, vec4<f32>(sum * 0.25, 0.0, 0.0));
}

@compute @workgroup_size(8, 8, 1)
fn cs_down(@builtin(global_invocation_id) id: vec3<u32>) {
    let texel = vec2<i32>(id.xy);
    if any(texel >= vec2<i32>(textureDimensions(out_level))) {
        return;
    }
    let size = vec2<i32>(textureDimensions(source_level));
    var sum = vec2<f32>(0.0);
    for (var i = 0; i < 4; i++) {
        sum += textureLoad(source_level, clamp_load(texel * 2, tap_offset(i), size), 0).xy;
    }
    textureStore(out_level, texel, vec4<f32>(sum * 0.25, 0.0, 0.0));
}
