// FSR 3.1 upscaler pass 5, prepare reactivity: for every render pixel, how much of its history can be
// trusted. It finds disocclusions from the reconstructed previous depth, combines the shading change
// and the application's masks, tracks how many frames a pixel has accumulated, and seeds locks on thin
// features. The four results go to one texture: reactive, disocclusion, shading change, accumulation.
//
// Ported to WGSL from AMD's ffx_fsr3upscaler_prepare_reactivity.h (FidelityFX SDK v1.1.4, FSR 3.1.4):
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
// Changes from the original: the reconstructed depth comes from a buffer (see prepare_inputs.wgsl);
// the accumulation is 32-bit float and is zero on the first frame after a reset instead of relying on
// a cleared texture; a flat neighbourhood returns no lock at once rather than dividing by zero; the
// masks outside their texture read as zero.

@group(0) @binding(1) var dilated_motion_vectors: texture_2d<f32>;
@group(0) @binding(2) var dilated_depth: texture_2d<f32>;
@group(0) @binding(3) var<storage, read> reconstructed_depth: array<u32>;
@group(0) @binding(4) var reactive_mask: texture_2d<f32>;
@group(0) @binding(5) var transparency_mask: texture_2d<f32>;
@group(0) @binding(6) var current_luma: texture_2d<f32>;
@group(0) @binding(7) var shading_change: texture_2d<f32>;
@group(0) @binding(8) var previous_accumulation: texture_2d<f32>;
@group(0) @binding(9) var<storage, read> frame_info: array<vec4<f32>>;
@group(0) @binding(10) var out_reactive_masks: texture_storage_2d<rgba16float, write>;
@group(0) @binding(11) var out_accumulation: texture_storage_2d<r32float, write>;
@group(0) @binding(12) var new_locks: texture_storage_2d<r32float, read_write>;

fn exposure() -> f32 {
    return frame_info[0].x;
}

// The previous frame's nearest depth at a pixel, or the far value where nothing was written.
fn load_reconstructed_prev_depth(pos: vec2<i32>) -> f32 {
    let key = reconstructed_depth[u32(pos.y * cb.render_size.x + pos.x)];
    if key == 0u {
        return select(1.0, 0.0, INVERTED_DEPTH);
    }
    if INVERTED_DEPTH {
        return bitcast<f32>(key);
    }
    return bitcast<f32>(0xFFFFFFFFu - key);
}

fn compute_disocclusions(uv: vec2<f32>, motion_vector_in: vec2<f32>, current_depth_view_space: f32) -> f32 {
    let nearest_in_meters = min(current_depth_view_space * cb.view_space_to_meters, FP16_MAX);
    let threshold = reconstructed_depth_mv_px_threshold(nearest_in_meters);
    var motion_vector = motion_vector_in;
    if get_4k_velocity(motion_vector) <= threshold {
        motion_vector = vec2<f32>(0.0);
    }

    let taps = get_bilinear_taps(uv + motion_vector, cb.render_size);
    let weights = bilinear_weights(taps.fraction);
    var disocclusion = 0.0;
    var weight_sum = 0.0;
    var potential_disocclusion = true;
    for (var i = 0; i < 4 && potential_disocclusion; i++) {
        let sample_pos = clamp_load(taps.base, tap_offset(i), cb.render_size);
        if weights[i] <= BILINEAR_WEIGHT_THRESHOLD {
            continue;
        }
        let previous_nearest = get_view_space_depth(load_reconstructed_prev_depth(sample_pos));
        let depth_difference = current_depth_view_space - previous_nearest;
        potential_disocclusion = potential_disocclusion && depth_difference > FP32_MIN;
        if potential_disocclusion {
            let half_viewport = length(vec2<f32>(cb.render_size) * 0.5);
            let depth_threshold = max(current_depth_view_space, previous_nearest);
            let ksep = 1.37e-05;
            let required_separation = ksep * half_viewport * depth_threshold;
            disocclusion += saturate(required_separation / depth_difference) * weights[i];
            weight_sum += weights[i];
        }
    }
    if potential_disocclusion && weight_sum > 0.0 {
        return saturate(1.0 - disocclusion / weight_sum);
    }
    return 0.0;
}

fn compute_motion_divergence(uv: vec2<f32>, motion_vector: vec2<f32>, current_depth: f32) -> f32 {
    let reprojected_pos = vec2<i32>((uv + motion_vector) * vec2<f32>(cb.render_size));
    let reprojected_depth = load_or_zero(dilated_depth, reprojected_pos).x;
    let reprojected_motion = load_or_zero(dilated_motion_vectors, reprojected_pos).xy;

    let reprojected_velocity = get_4k_velocity(reprojected_motion);
    let velocity = get_4k_velocity(motion_vector);
    if velocity <= 0.0 {
        return 0.0;
    }
    let nucleus_in_meters = get_view_space_depth_in_meters(reprojected_depth);
    let current_in_meters = get_view_space_depth_in_meters(current_depth);
    let distance_factor = min_divided_by_max(nucleus_in_meters, current_in_meters, 0.0);
    let velocity_factor = saturate(velocity / 10.0);
    return (1.0 - saturate(reprojected_velocity / velocity)) * distance_factor * velocity_factor;
}

fn dilate_reactive_masks(pos: vec2<i32>) -> f32 {
    var dilated = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let sample_pos = clamp_load(pos, vec2<i32>(x, y), cb.render_size);
            dilated = max(dilated, load_or_zero(reactive_mask, sample_pos).x * cb.reactiveness_scale);
        }
    }
    return dilated;
}

fn dilate_transparency_and_composition(uv: vec2<f32>) -> f32 {
    let size = vec2<i32>(textureDimensions(transparency_mask));
    return sample_linear(transparency_mask, clamp_uv(uv, cb.render_size, size)).x;
}

// A thin bright or dark line, one that stands out from all of its neighbours in the same direction,
// gets a lock that keeps its history through the jitter.
fn compute_thin_feature_confidence(pos: vec2<i32>) -> f32 {
    var offsets = array<vec2<i32>, 9>(
        vec2<i32>(0, 0), vec2<i32>(-1, -1), vec2<i32>(0, -1), vec2<i32>(1, -1), vec2<i32>(-1, 0),
        vec2<i32>(1, 0), vec2<i32>(-1, 1), vec2<i32>(0, 1), vec2<i32>(1, 1),
    );
    var samples: array<f32, 9>;
    var luma_min = FP32_MAX;
    var luma_max = FP32_MIN;
    for (var i = 0; i < 9; i++) {
        let sample_pos = clamp_load(pos, offsets[i], cb.render_size);
        samples[i] = textureLoad(current_luma, sample_pos, 0).x * exposure();
        luma_min = min(luma_min, samples[i]);
        luma_max = max(luma_max, samples[i]);
    }
    let luma_range = luma_max - luma_min;
    if !(luma_range > 0.0) {
        return 0.0;
    }

    let threshold = 0.9;
    var dissimilar_min = FP32_MAX;
    var dissimilar_max = 0.0;
    var pattern_mask = 1u;
    for (var i = 1; i < 9; i++) {
        let difference = abs(samples[i] - samples[0]) / luma_range;
        if difference < threshold {
            pattern_mask |= 1u << u32(i);
        } else {
            dissimilar_min = min(dissimilar_min, samples[i]);
            dissimilar_max = max(dissimilar_max, samples[i]);
        }
    }
    let is_ridge = samples[0] > dissimilar_max || samples[0] < dissimilar_min;
    if !is_ridge {
        return 0.0;
    }
    // Reject a ridge that is the corner of an edge: the sample and three of its neighbours alike.
    var rejection_masks = array<u32, 4>(
        (1u << 1u) | (1u << 2u) | (1u << 4u) | 1u,
        (1u << 2u) | (1u << 3u) | (1u << 5u) | 1u,
        (1u << 4u) | (1u << 6u) | (1u << 7u) | 1u,
        (1u << 5u) | (1u << 7u) | (1u << 8u) | 1u,
    );
    for (var i = 0; i < 4; i++) {
        if (pattern_mask & rejection_masks[i]) == rejection_masks[i] {
            return 0.0;
        }
    }
    return 1.0 - luma_min / luma_max;
}

// The number of frames of history a pixel has, in units of `accumulation_added_per_frame`.
fn update_accumulation(pos: vec2<i32>, uv: vec2<f32>, motion_vector: vec2<f32>, disocclusion: f32, shading: f32) -> f32 {
    let reprojected = uv + motion_vector;
    var accumulation = 0.0;
    if is_uv_inside(reprojected) && frame_index() > 0.0 {
        let reprojected_hw = clamp_uv(reprojected, cb.previous_render_size, cb.max_render_size);
        accumulation = saturate(sample_linear(previous_accumulation, reprojected_hw).x);
    }
    // A shading change drops the accumulation; a disocclusion restarts it below zero.
    accumulation = mix(accumulation, 0.0, shading);
    accumulation = mix(accumulation, min(cb.min_disocclusion_accumulation, accumulation), disocclusion);
    accumulation *= f32(round(accumulation * 100.0) > 1.0);

    textureStore(out_accumulation, pos, vec4<f32>(saturate(accumulation + cb.accumulation_added_per_frame), 0.0, 0.0, 0.0));
    return accumulation;
}

fn compute_shading_change(uv: vec2<f32>) -> f32 {
    // The jitter is applied again here and taken off when the value is read in the accumulate pass.
    let shading_uv = clamp_uv(uv - cb.jitter / vec2<f32>(cb.render_size), shading_change_render_size(), vec2<i32>(textureDimensions(shading_change)));
    return saturate(sample_linear(shading_change, shading_uv).x * cb.shading_change_scale);
}

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pos = vec2<i32>(id.xy);
    if !is_on_screen(pos, cb.render_size) {
        return;
    }
    let uv = (vec2<f32>(pos) + vec2<f32>(0.5)) / vec2<f32>(cb.render_size);
    let motion_vector = textureLoad(dilated_motion_vectors, pos, 0).xy;
    let depth = textureLoad(dilated_depth, pos, 0).x;

    let disocclusion = compute_disocclusions(uv, motion_vector, get_view_space_depth(depth));
    let shading = max(dilate_reactive_masks(pos), compute_shading_change(uv));
    let divergence = compute_motion_divergence(uv, motion_vector, depth);
    let transparency = dilate_transparency_and_composition(uv);
    let reactiveness = max(divergence, transparency);
    let accumulation = update_accumulation(pos, uv, motion_vector, disocclusion, shading);

    textureStore(out_reactive_masks, pos, vec4<f32>(reactiveness, disocclusion, shading, accumulation));

    let lock_strength = compute_thin_feature_confidence(pos);
    if lock_strength > (1.0 / 100.0) {
        let lock_pos = compute_hr_pos_from_lr_pos(pos);
        if is_on_screen(lock_pos, cb.upscale_size) {
            textureStore(new_locks, lock_pos, vec4<f32>(lock_strength, 0.0, 0.0, 0.0));
        }
    }
}
