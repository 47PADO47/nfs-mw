// FSR 3.1 upscaler pass 6, luma instability: looks at the last four frames of a pixel's luma, and when
// the new value flips back towards an older one (a flickering pixel, not a real change) reports it, so
// that the accumulate pass can keep trusting the history there.
//
// Ported to WGSL from AMD's ffx_fsr3upscaler_luma_instability.h (FidelityFX SDK v1.1.4, FSR 3.1.4):
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
// Changes from the original: the unused farthest-depth input is gone, the luma history is clamped
// to the range of its 16-bit storage, and the texels are filtered by hand.

@group(0) @binding(1) var dilated_motion_vectors: texture_2d<f32>;
@group(0) @binding(2) var reactive_masks: texture_2d<f32>;
@group(0) @binding(3) var current_luma: texture_2d<f32>;
@group(0) @binding(4) var previous_luma_history: texture_2d<f32>;
@group(0) @binding(5) var<storage, read> frame_info: array<vec4<f32>>;
@group(0) @binding(6) var out_luma_history: texture_storage_2d<rgba16float, write>;
@group(0) @binding(7) var out_luma_instability: texture_storage_2d<r32float, write>;

fn exposure() -> f32 {
    return frame_info[0].x;
}

struct LumaInstability {
    history: vec4<f32>,
    factor: f32,
}

// `history` holds the luma of frames N-1 to N-4, already scaled to this frame's exposure.
fn compute_luma_instability_factor(history_in: vec4<f32>, current_frame_luma: f32) -> LumaInstability {
    var history = history_in;
    var instability = 0.0;
    let diff0 = current_frame_luma - history[0];
    let similarity0 = min_divided_by_max(current_frame_luma, history[0], 1.0);
    var max_similarity = similarity0;
    if similarity0 < 1.0 {
        for (var i = 1; i <= 3; i++) {
            let diff1 = current_frame_luma - history[i];
            let similarity1 = min_divided_by_max(current_frame_luma, history[i], 0.0);
            if sign(diff0) == sign(diff1) {
                max_similarity = max(max_similarity, similarity1);
            }
        }
        instability = f32(max_similarity > similarity0);
    }

    // Shift the history and divide the exposure out again.
    history = vec4<f32>(current_frame_luma, history.x, history.y, history.z) / exposure();
    return LumaInstability(history, instability * f32(history.w != 0.0));
}

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pos = vec2<i32>(id.xy);
    if !is_on_screen(pos, cb.render_size) {
        return;
    }
    var data = LumaInstability(vec4<f32>(0.0), 0.0);

    let motion_vector = textureLoad(dilated_motion_vectors, pos, 0).xy;
    let uv = (vec2<f32>(pos) + vec2<f32>(0.5)) / vec2<f32>(cb.render_size);
    let uv_current_jittered = uv + cb.jitter / vec2<f32>(cb.render_size);
    let uv_previous_jittered = uv + cb.previous_jitter / vec2<f32>(cb.previous_render_size);
    let reprojected = uv_previous_jittered + motion_vector;

    if is_uv_inside(reprojected) {
        let uv_hw = clamp_uv(uv_current_jittered, cb.render_size, cb.max_render_size);
        let masks = sample_linear(reactive_masks, uv_hw);
        let reactive = saturate(masks[MASK_REACTIVE]);
        let disocclusion = saturate(masks[MASK_DISOCCLUSION]);
        let shading_change = saturate(masks[MASK_SHADING_CHANGE]);
        let accumulation = saturate(masks[MASK_ACCUMULATION]);

        // Only pixels with a settled history can be told to flicker.
        if accumulation > 0.9 {
            let luma = sample_linear(current_luma, uv_hw).x * exposure();
            let reprojected_hw = clamp_uv(reprojected, cb.previous_render_size, cb.max_render_size);
            data.history = sample_linear(previous_luma_history, reprojected_hw) * cb.delta_pre_exposure * exposure();
            data = compute_luma_instability_factor(data.history, luma);

            let velocity_weight = 1.0 - saturate(get_4k_velocity(motion_vector) / 20.0);
            data.factor *= velocity_weight * (1.0 - disocclusion) * (1.0 - reactive) * (1.0 - shading_change);
        }
    }

    textureStore(out_luma_history, pos, clamp(data.history, vec4<f32>(0.0), vec4<f32>(FP16_MAX)));
    textureStore(out_luma_instability, pos, vec4<f32>(data.factor, 0.0, 0.0, 0.0));
}
