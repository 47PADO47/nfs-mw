// FSR 3.1 upscaler pass 1, prepare inputs: finds the nearest and farthest depth in each 3x3 block,
// dilates the motion vectors to the nearest depth, reconstructs the previous frame's nearest depth by
// pushing the current depth along the motion vectors, and stores the luma of the colour.
//
// Ported to WGSL from AMD's ffx_fsr3upscaler_prepare_inputs.h (FidelityFX SDK v1.1.4, FSR 3.1.4):
// https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK
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
// Changes from the original: the reconstructed depth is a buffer of atomics holding the depth's bits
// (a storage texture cannot be atomic everywhere), stored as the bits for reverse depth and as their
// complement for standard depth so that both use atomicMax and a zeroed buffer means "nothing written".

@group(0) @binding(1) var input_color: texture_2d<f32>;
@group(0) @binding(2) var input_depth: texture_2d<f32>;
@group(0) @binding(3) var input_motion_vectors: texture_2d<f32>;
@group(0) @binding(4) var out_dilated_motion_vectors: texture_storage_2d<rg32float, write>;
@group(0) @binding(5) var out_dilated_depth: texture_storage_2d<r32float, write>;
@group(0) @binding(6) var out_farthest_depth: texture_storage_2d<r32float, write>;
@group(0) @binding(7) var out_current_luma: texture_storage_2d<r32float, write>;
@group(0) @binding(8) var<storage, read_write> reconstructed_depth: array<atomic<u32>>;

fn load_depth(pos: vec2<i32>) -> f32 {
    return textureLoad(input_depth, clamp_load(pos, vec2<i32>(0), cb.render_size), 0).x;
}

// The motion vector in uv units; the texture is read at its own resolution.
fn load_input_motion_vector(pos: vec2<i32>) -> vec2<f32> {
    let size = vec2<i32>(textureDimensions(input_motion_vectors));
    var motion = textureLoad(input_motion_vectors, clamp_load(pos, vec2<i32>(0), size), 0).xy * cb.motion_vector_scale;
    if JITTERED_MOTION_VECTORS {
        motion -= cb.motion_vector_jitter_cancellation;
    }
    return motion;
}

// Records `depth` as the previous frame's depth at `pos`, keeping the nearest one per pixel.
fn store_reconstructed_depth(pos: vec2<i32>, depth: f32) {
    let bits = bitcast<u32>(clamp(depth, 0.0, 1.0));
    var key = bits;
    if !INVERTED_DEPTH {
        key = 0xFFFFFFFFu - bits;
    }
    atomicMax(&reconstructed_depth[u32(pos.y * cb.render_size.x + pos.x)], key);
}

fn reconstruct_prev_depth(pos: vec2<i32>, depth: f32, motion_vector_in: vec2<f32>) {
    let nearest_in_meters = min(get_view_space_depth_in_meters(depth), FP16_MAX);
    let threshold = reconstructed_depth_mv_px_threshold(nearest_in_meters);

    // Discard small motion.
    var motion_vector = motion_vector_in;
    if get_4k_velocity(motion_vector) <= threshold {
        motion_vector = vec2<f32>(0.0);
    }

    let uv = (vec2<f32>(pos) + vec2<f32>(0.5)) / vec2<f32>(cb.render_size);
    let taps = get_bilinear_taps(uv + motion_vector, cb.render_size);
    let weights = bilinear_weights(taps.fraction);

    // Project the current depth into the previous frame's pixels, to every pixel that bilinear
    // reprojection would take a contribution from.
    for (var i = 0; i < 4; i++) {
        let store_pos = taps.base + tap_offset(i);
        if weights[i] > BILINEAR_WEIGHT_THRESHOLD && is_on_screen(store_pos, cb.render_size) {
            store_reconstructed_depth(store_pos, depth);
        }
    }
}

struct DepthExtents {
    nearest: f32,
    nearest_coord: vec2<i32>,
    farthest: f32,
}

// Whether depth `a` is nearer to the camera than `b`.
fn is_nearer(a: f32, b: f32) -> bool {
    if INVERTED_DEPTH {
        return a > b;
    }
    return a < b;
}

// The depth farther from the camera. AMD takes this from a sample that is nearer than the nearest so far:
// reverse depth keeps the smaller value, standard depth the larger one.
fn farther_of(a: f32, b: f32) -> f32 {
    if INVERTED_DEPTH {
        return min(a, b);
    }
    return max(a, b);
}

fn find_depth_extents(pos: vec2<i32>) -> DepthExtents {
    var offsets = array<vec2<i32>, 9>(
        vec2<i32>(0, 0), vec2<i32>(1, 0), vec2<i32>(0, 1), vec2<i32>(0, -1), vec2<i32>(-1, 0),
        vec2<i32>(-1, 1), vec2<i32>(1, 1), vec2<i32>(-1, -1), vec2<i32>(1, -1),
    );
    let center = load_depth(pos);
    var extents = DepthExtents(center, pos, center);
    for (var i = 1; i < 9; i++) {
        let sample_pos = pos + offsets[i];
        if !is_on_screen(sample_pos, cb.render_size) {
            continue;
        }
        let depth = load_depth(sample_pos);
        if !is_nearer(depth, extents.nearest) {
            continue;
        }
        extents.farthest = farther_of(extents.farthest, depth);
        extents.nearest_coord = sample_pos;
        extents.nearest = depth;
    }
    return extents;
}

fn dilate_motion_vector(extents: DepthExtents) -> vec2<f32> {
    if LOW_RES_MOTION_VECTORS {
        return load_input_motion_vector(extents.nearest_coord);
    }
    return load_input_motion_vector(compute_hr_pos_from_lr_pos(extents.nearest_coord));
}

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pos = vec2<i32>(id.xy);
    if !is_on_screen(pos, cb.render_size) {
        return;
    }
    let extents = find_depth_extents(pos);
    let dilated_motion = dilate_motion_vector(extents);

    reconstruct_prev_depth(pos, extents.nearest, dilated_motion);

    textureStore(out_dilated_motion_vectors, pos, vec4<f32>(dilated_motion, 0.0, 0.0));
    textureStore(out_dilated_depth, pos, vec4<f32>(extents.nearest, 0.0, 0.0, 0.0));

    let farthest_in_meters = min(get_view_space_depth_in_meters(extents.farthest), FP16_MAX);
    textureStore(out_farthest_depth, pos, vec4<f32>(farthest_in_meters, 0.0, 0.0, 0.0));

    // The colour is linear: luma needs no conversion.
    let rgb = sanitize_color(textureLoad(input_color, pos, 0).rgb);
    textureStore(out_current_luma, pos, vec4<f32>(rgb_to_luma(rgb), 0.0, 0.0, 0.0));
}
