// FSR 3.1 upscaler pass 7, accumulate: for every output pixel, reprojects the previous output with a
// Lanczos filter, upsamples the jittered render pixels around it with a Lanczos kernel, rectifies the
// history against the colours of the neighbourhood, blends the two by the confidence of the history
// and stores the result as the new history and, unless sharpening follows, as the output.
//
// Ported to WGSL from AMD's ffx_fsr3upscaler_accumulate.h, ffx_fsr3upscaler_upsample.h,
// ffx_fsr3upscaler_reproject.h and ffx_fsr3upscaler_sample.h (FidelityFX SDK v1.1.4, FSR 3.1.4):
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
// Changes from the original: only the fp32 reference path (the approximate Lanczos for the upsample
// kernel and the sin-based one for the history), without the paired-16-bit Xbox variant; the history
// taps repeat the edge texel instead of AMD's clamp to 1..size-2; the history and the new locks are
// read with textureLoad; the history is clamped to the range of its 16-bit
// storage; `OUTPUT_FORMAT` is replaced with the format of the output texture when the module is built.

@group(0) @binding(1) var input_color: texture_2d<f32>;
@group(0) @binding(2) var input_motion_vectors: texture_2d<f32>;
@group(0) @binding(3) var dilated_motion_vectors: texture_2d<f32>;
@group(0) @binding(4) var reactive_masks: texture_2d<f32>;
@group(0) @binding(5) var previous_history: texture_2d<f32>;
@group(0) @binding(6) var farthest_depth_mip1: texture_2d<f32>;
@group(0) @binding(7) var luma_instability: texture_2d<f32>;
@group(0) @binding(8) var<storage, read> frame_info: array<vec4<f32>>;
@group(0) @binding(9) var out_history: texture_storage_2d<rgba16float, write>;
@group(0) @binding(10) var out_upscaled: texture_storage_2d<OUTPUT_FORMAT, write>;
@group(0) @binding(11) var new_locks: texture_storage_2d<r32float, read_write>;

const PI: f32 = 3.141592653589793;

fn exposure() -> f32 {
    return frame_info[0].x;
}

// ---- Lanczos filters (ffx_fsr3upscaler_sample.h) -------------------------------------------------

fn lanczos2(x_in: f32) -> f32 {
    let x = min(abs(x_in), 2.0);
    if x < EPSILON {
        return 1.0;
    }
    return (sin(PI * x) / (PI * x)) * (sin(0.5 * PI * x) / (0.5 * PI * x));
}

// FSR 1's approximation of Lanczos2; the input is x * x.
fn lanczos2_approx_sq(x2_in: f32) -> f32 {
    let x2 = min(x2_in, 4.0);
    let a = (2.0 / 5.0) * x2 - 1.0;
    let b = (1.0 / 4.0) * x2 - 1.0;
    return ((25.0 / 16.0) * a * a - (25.0 / 16.0 - 1.0)) * (b * b);
}

fn lanczos2_weights(t: f32) -> vec4<f32> {
    return vec4<f32>(lanczos2(-1.0 - t), lanczos2(-t), lanczos2(1.0 - t), lanczos2(2.0 - t));
}

// AMD clamps the taps to 1..size-2, which reads the second texel in place of the first one and shifts
// the picture by a pixel along the edges of the screen; the taps here repeat the edge texel instead.
fn history_texel(pos: vec2<i32>, size: vec2<i32>) -> vec4<f32> {
    return textureLoad(previous_history, clamp_load(pos, vec2<i32>(0), size), 0);
}

// The previous output at `uv`: a 4x4 Lanczos2 filter with the deringing of AMD's reference.
fn history_sample(uv: vec2<f32>, size: vec2<i32>) -> vec4<f32> {
    var px = uv * vec2<f32>(size) - vec2<f32>(0.5);
    let fraction = fract(px);
    px = clamp(px, vec2<f32>(0.0), vec2<f32>(size) - vec2<f32>(1.0));
    let base = vec2<i32>(floor(px));
    let wx = lanczos2_weights(fraction.x);
    let wy = lanczos2_weights(fraction.y);

    var total = vec4<f32>(0.0);
    var ring_min = vec4<f32>(FP32_MAX);
    var ring_max = vec4<f32>(-FP32_MAX);
    for (var y = 0; y < 4; y++) {
        var row = vec4<f32>(0.0);
        for (var x = 0; x < 4; x++) {
            let texel = history_texel(base + vec2<i32>(x - 1, y - 1), size);
            row += wx[x] * texel;
            if x >= 1 && x <= 2 && y >= 1 && y <= 2 {
                ring_min = min(ring_min, texel);
                ring_max = max(ring_max, texel);
            }
        }
        total += wy[y] * row;
    }
    let filtered = total / (dot(wx, vec4<f32>(1.0)) * dot(wy, vec4<f32>(1.0)));
    return clamp(filtered, ring_min, ring_max);
}

// ---- State of the pass (AccumulationPassCommonParams / AccumulationPassData) ---------------------

struct Params {
    hr_pos: vec2<i32>,
    hr_uv: vec2<f32>,
    lr_uv_jittered: vec2<f32>,
    lr_uv_hw: vec2<f32>,
    motion_vector: vec2<f32>,
    reprojected_hr_uv: vec2<f32>,
    velocity_4k: f32,
    disocclusion: f32,
    reactive: f32,
    shading_change: f32,
    accumulation: f32,
    luma_instability: f32,
    farthest_depth_in_meters: f32,
    is_existing_sample: bool,
    is_new_sample: bool,
}

struct RectificationBox {
    center: vec3<f32>,
    vec_std: vec3<f32>,
    aabb_min: vec3<f32>,
    aabb_max: vec3<f32>,
}

// The upsampled render pixels around an output pixel, as YCoCg: the filtered colour and its weight,
// plus the colour statistics used to rectify the history.
struct Upsample {
    color: vec3<f32>,
    weight: f32,
    history_weight: f32,
    rbox: RectificationBox,
}

fn load_input_motion_vector(pos: vec2<i32>) -> vec2<f32> {
    let size = vec2<i32>(textureDimensions(input_motion_vectors));
    var motion = textureLoad(input_motion_vectors, clamp_load(pos, vec2<i32>(0), size), 0).xy * cb.motion_vector_scale;
    if JITTERED_MOTION_VECTORS {
        motion -= cb.motion_vector_jitter_cancellation;
    }
    return motion;
}

fn get_motion_vector(hr_pos: vec2<i32>, hr_uv: vec2<f32>) -> vec2<f32> {
    if LOW_RES_MOTION_VECTORS {
        return textureLoad(dilated_motion_vectors, vec2<i32>(hr_uv * vec2<f32>(cb.render_size)), 0).xy;
    }
    return load_input_motion_vector(hr_pos);
}

fn init_params(hr_pos: vec2<i32>) -> Params {
    var p: Params;
    p.hr_pos = hr_pos;
    p.hr_uv = (vec2<f32>(hr_pos) + vec2<f32>(0.5)) / vec2<f32>(cb.upscale_size);
    p.lr_uv_jittered = p.hr_uv + cb.jitter / vec2<f32>(cb.render_size);
    p.lr_uv_hw = clamp_uv(p.lr_uv_jittered, cb.render_size, cb.max_render_size);
    p.motion_vector = get_motion_vector(hr_pos, p.hr_uv);
    p.velocity_4k = get_4k_velocity(p.motion_vector);
    p.reprojected_hr_uv = p.hr_uv + p.motion_vector;
    p.is_existing_sample = is_uv_inside(p.reprojected_hr_uv);

    let instability_uv = clamp_uv(p.hr_uv, cb.render_size, cb.max_render_size);
    p.luma_instability = sample_linear(luma_instability, instability_uv).x;
    let farthest_uv = clamp_uv(p.lr_uv_jittered, cb.render_size / 2, vec2<i32>(textureDimensions(farthest_depth_mip1)));
    p.farthest_depth_in_meters = sample_linear(farthest_depth_mip1, farthest_uv).x;
    p.is_new_sample = !p.is_existing_sample || cb.frame_index == 0.0;

    let masks = sample_linear(reactive_masks, p.lr_uv_hw);
    p.reactive = saturate(masks[MASK_REACTIVE]);
    p.disocclusion = saturate(masks[MASK_DISOCCLUSION]);
    p.shading_change = saturate(masks[MASK_SHADING_CHANGE]);
    p.accumulation = saturate(masks[MASK_ACCUMULATION]);
    p.accumulation *= f32(round(p.accumulation * 100.0) > 1.0);
    return p;
}

// Tracks which pixels to keep the history of, for thin features (the `new_locks` of the render pixels
// that sampled them). Returns [lock, the lock's contribution to rectification].
fn update_lock_status(p: Params, lock_in: f32) -> vec2<f32> {
    var lock = lock_in * f32(!p.is_new_sample);
    let lifetime_decrease_factor = max(saturate(p.shading_change), max(p.reactive, p.disocclusion));
    lock = max(0.0, lock - lifetime_decrease_factor * LOCK_MAX);
    let contribution = saturate(saturate(lock - LOCK_THRESHOLD) * (LOCK_MAX - LOCK_THRESHOLD));

    let new_lock = textureLoad(new_locks, p.hr_pos).x * (1.0 - p.reactive);
    lock = max(0.0, min(lock + new_lock, LOCK_MAX));

    // Locks fade over a jitter cycle, and die where the pixel is about to leave the screen.
    lock = max(0.0, lock - (0.1 / cb.jitter_phase_count) * (1.0 - lifetime_decrease_factor));
    lock *= f32(is_uv_inside(p.hr_uv - p.motion_vector));
    return vec2<f32>(lock, contribution);
}

fn base_accumulation_weight(p: Params) -> f32 {
    var base = p.accumulation;
    base = min(base, mix(base, 0.15, saturate(max(0.0, (p.velocity_4k * cb.velocity_factor) / 0.5))));
    return base;
}

// ---- Upsampling (ffx_fsr3upscaler_upsample.h) ----------------------------------------------------

fn load_prepared_color(pos: vec2<i32>) -> vec3<f32> {
    let rgb = sanitize_color(textureLoad(input_color, pos, 0).rgb) * exposure();
    return rgb_to_ycocg(rgb);
}

fn compute_upsampled_color_and_weight(p: Params, base_history_weight: f32) -> Upsample {
    var up: Upsample;
    up.history_weight = base_history_weight;
    up.color = vec3<f32>(0.0);
    up.weight = 0.0;

    // A sliced Lanczos filter with two lobes: the other slices are accumulated over time.
    let dst_output_pos = vec2<f32>(p.hr_pos) + vec2<f32>(0.5);
    let src_output_pos = dst_output_pos * cb.downscale_factor;
    let src_input_pos = vec2<i32>(floor(src_output_pos));
    let src_unjittered_pos = vec2<f32>(src_input_pos) + vec2<f32>(0.5) - cb.jitter;
    let base_sample_offset = src_unjittered_pos - src_output_pos;
    let flip_col = src_unjittered_pos.x > src_output_pos.x;
    let flip_row = src_unjittered_pos.y > src_output_pos.y;
    let offset_tl = vec2<i32>(select(-1, -2, flip_col), select(-1, -2, flip_row));
    let is_initial_sample = p.accumulation == 0.0;

    var samples: array<vec3<f32>, 9>;
    for (var row = 0; row < 3; row++) {
        for (var col = 0; col < 3; col++) {
            let col_row = vec2<i32>(select(col, 3 - col, flip_col), select(row, 3 - row, flip_row));
            let coord = clamp_load(src_input_pos + offset_tl + col_row, vec2<i32>(0), cb.render_size);
            samples[row * 3 + col] = load_prepared_color(coord);
        }
    }
    if HDR_INPUT && is_initial_sample {
        // Initial samples are tone mapped (through RGB, to avoid desaturating).
        for (var i = 0; i < 9; i++) {
            samples[i] = rgb_to_ycocg(tonemap(ycocg_to_rgb(samples[i])));
        }
    }

    // How much of each upsampled colour to use this frame.
    let kernel_bias_max = min(1.99, 1.0 / cb.downscale_factor.x);
    let kernel_bias_min = max(1.0, (1.0 + kernel_bias_max) * 0.3);
    let kernel_bias_weight = min(1.0 - p.disocclusion * 0.5, min(1.0 - p.shading_change, saturate(up.history_weight * 5.0)));
    let kernel_bias = mix(kernel_bias_min, kernel_bias_max, kernel_bias_weight);

    var rbox: RectificationBox;
    var box_weight = 0.0;
    var box_vec = vec3<f32>(0.0);
    for (var row = 0; row < 3; row++) {
        for (var col = 0; col < 3; col++) {
            let index = row * 3 + col;
            let col_row = vec2<i32>(select(col, 3 - col, flip_col), select(row, 3 - row, flip_row));
            let src_sample_offset = base_sample_offset + vec2<f32>(offset_tl + col_row);
            let src_sample_pos = src_input_pos + offset_tl + col_row;
            let on_screen = f32(is_on_screen(src_sample_pos, cb.render_size));
            let color = samples[index];

            if !is_initial_sample {
                let biased = src_sample_offset * kernel_bias;
                let weight = on_screen * lanczos2_approx_sq(dot(biased, biased));
                up.color += color * weight;
                up.weight += weight;
            }

            // Statistics of the neighbourhood for rectification.
            let curve_bias = -2.3;
            let box_sample_weight = exp(curve_bias * dot(src_sample_offset, src_sample_offset)) * on_screen;
            let weighted = color * box_sample_weight;
            if index == 0 {
                rbox.aabb_min = color;
                rbox.aabb_max = color;
                rbox.center = weighted;
                box_vec = color * weighted;
                box_weight = box_sample_weight;
            } else {
                rbox.aabb_min = min(rbox.aabb_min, color);
                rbox.aabb_max = max(rbox.aabb_max, color);
                rbox.center += weighted;
                box_vec += color * weighted;
                box_weight += box_sample_weight;
            }
        }
    }
    // Variance box: the weighted mean and standard deviation.
    box_weight = select(1.0, box_weight, abs(box_weight) > FP32_MIN);
    rbox.center /= box_weight;
    box_vec /= box_weight;
    rbox.vec_std = sqrt(abs(box_vec - rbox.center * rbox.center));
    up.rbox = rbox;

    up.weight *= f32(up.weight > EPSILON);
    if up.weight > EPSILON {
        // Normalise for deringing, which compares colours.
        up.color /= up.weight;
        up.weight *= AVERAGE_LANCZOS_WEIGHT_PER_FRAME;
        up.color = clamp(up.color, rbox.aabb_min, rbox.aabb_max);
    }

    // Initial samples use the tone mapped upsampled colour.
    if is_initial_sample {
        up.color = rbox.center;
        if HDR_INPUT {
            up.color = rgb_to_ycocg(inverse_tonemap(ycocg_to_rgb(rbox.center)));
        }
        up.weight = 1.0;
        up.history_weight = 0.0;
    }
    return up;
}

// ---- Rectification and blending (ffx_fsr3upscaler_accumulate.h) ----------------------------------

fn rectify_history(p: Params, up: Upsample, history_in: vec3<f32>, lock_contribution: f32) -> vec3<f32> {
    let velocity_factor = saturate(p.velocity_4k / 20.0);
    let distance_factor = saturate(0.75 - p.farthest_depth_in_meters / 20.0);
    let accumulation_factor = 1.0 - p.accumulation;
    let reactive_factor = pow(p.reactive, 0.5);
    let shading_change_factor = p.shading_change;
    let box_scale_t = max(velocity_factor, max(distance_factor, max(accumulation_factor, max(reactive_factor, shading_change_factor))));

    let box_scale = mix(3.0, 1.0, box_scale_t);
    let scaled_box_vec = up.rbox.vec_std * vec3<f32>(1.7, 1.0, 1.0) * box_scale;
    let clamped_scaled_box_vec = max(scaled_box_vec, vec3<f32>(1.193e-7));
    let transformed = (history_in - up.rbox.center) / clamped_scaled_box_vec;

    var history = history_in;
    if length(transformed) > 1.0 {
        let clamped_history = normalize(transformed) * scaled_box_vec + up.rbox.center;
        // Scale the history using the rectification info, also using the accumulation mask to avoid
        // an invalid-colour protection.
        let contribution = max(p.luma_instability, lock_contribution) * p.accumulation * (1.0 - p.disocclusion);
        history = mix(clamped_history, history, saturate(contribution));
    }
    return history;
}

// Blends the history with the upsampled colour, in tone mapped space for HDR input.
fn blend_history(up: Upsample, history_color: vec3<f32>, history_weight_in: f32) -> vec3<f32> {
    var history_weight = history_weight_in * f32(history_weight_in > FP16_MIN);
    history_weight = max(EPSILON, history_weight + up.weight);

    var upsampled = up.color;
    var history = history_color;
    if HDR_INPUT {
        upsampled = rgb_to_ycocg(tonemap(ycocg_to_rgb(upsampled)));
        history = rgb_to_ycocg(tonemap(ycocg_to_rgb(history)));
    }
    let alpha = saturate(up.weight / history_weight);
    var blended = ycocg_to_rgb(mix(history, upsampled, alpha));
    if HDR_INPUT {
        blended = inverse_tonemap(blended);
    }
    return blended;
}

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let hr_pos = vec2<i32>(id.xy);
    if !is_on_screen(hr_pos, cb.upscale_size) {
        return;
    }
    let p = init_params(hr_pos);

    var history_color = vec3<f32>(0.0);
    var lock = 0.0;
    if p.is_existing_sample && !p.is_new_sample {
        // Reproject the history into this frame's exposure.
        let reprojected = history_sample(p.reprojected_hr_uv, cb.previous_upscale_size);
        history_color = rgb_to_ycocg(reprojected.rgb * cb.delta_pre_exposure * exposure());
        lock = reprojected.w;
    }
    let lock_status = update_lock_status(p, lock);
    let up = compute_upsampled_color_and_weight(p, base_accumulation_weight(p));
    history_color = rectify_history(p, up, history_color, lock_status.y);
    var result = blend_history(up, history_color, up.history_weight) / exposure();
    // Both the history and the usual outputs are 16-bit floats.
    result = clamp(result, vec3<f32>(0.0), vec3<f32>(FP16_MAX));

    textureStore(out_history, hr_pos, vec4<f32>(result, lock_status.x));
    if !APPLY_SHARPENING {
        textureStore(out_upscaled, hr_pos, vec4<f32>(result, 1.0));
    }
    textureStore(new_locks, hr_pos, vec4<f32>(0.0));
}
