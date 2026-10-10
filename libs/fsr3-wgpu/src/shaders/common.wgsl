// FidelityFX Super Resolution 3.1 upscaler, shared definitions: constants, the uniform block, view-space
// depth, colour spaces and the texture helpers every pass uses. Ported to WGSL from AMD's
// ffx_fsr3upscaler_common.h and ffx_fsr3upscaler_callbacks_hlsl.h (FidelityFX SDK v1.1.4, FSR 3.1.4):
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
// Changes from the original: fp32 only, no subgroup operations, no 16-bit paths; textures are read
// with textureLoad and filtered by hand (bilinear taps below), so no sampler is bound and 32-bit float
// textures need no filtering support; the options of the HLSL permutations are pipeline-overridable
// constants. This file is placed in front of every pass: each pass file declares its own bindings from
// binding 1 on. Binding 0 is the uniform block.

override INVERTED_DEPTH: bool = false;
override HDR_INPUT: bool = true;
// Motion vectors at the render resolution (otherwise at the output resolution).
override LOW_RES_MOTION_VECTORS: bool = true;
// Motion vectors that include the jitter of both frames.
override JITTERED_MOTION_VECTORS: bool = false;
// The exposure is computed by the luma pyramid pass (otherwise an input texture holds it).
override AUTO_EXPOSURE: bool = true;
// The accumulate pass leaves the output to the RCAS pass (otherwise it writes it itself).
override APPLY_SHARPENING: bool = false;

const FP16_MIN: f32 = 6.10e-05;
const FP16_MAX: f32 = 65504.0;
const EPSILON: f32 = 6.10e-05;
const TONEMAP_EPSILON: f32 = 6.10e-05;
const FP32_MAX: f32 = 3.402823466e+38;
const FP32_MIN: f32 = 1.175494351e-38;
// Colours are clamped to this on load so a stray huge value cannot poison the sums and histories.
const COLOR_MAX: f32 = 1.0e7;

const BILINEAR_WEIGHT_THRESHOLD: f32 = EPSILON * 10.0;
const UPSAMPLE_LANCZOS_WEIGHT_SCALE: f32 = 1.0 / 16.0;
const AVERAGE_LANCZOS_WEIGHT_PER_FRAME: f32 = 0.74 * UPSAMPLE_LANCZOS_WEIGHT_SCALE;
const SHADING_CHANGE_SET_SIZE: i32 = 5;
const LOCK_THRESHOLD: f32 = 1.0;
const LOCK_MAX: f32 = 2.0;

// Channels of the dilated reactive mask texture.
const MASK_REACTIVE: i32 = 0;
const MASK_DISOCCLUSION: i32 = 1;
const MASK_SHADING_CHANGE: i32 = 2;
const MASK_ACCUMULATION: i32 = 3;

struct Constants {
    render_size: vec2<i32>,
    previous_render_size: vec2<i32>,
    upscale_size: vec2<i32>,
    previous_upscale_size: vec2<i32>,
    max_render_size: vec2<i32>,
    max_upscale_size: vec2<i32>,
    device_to_view_depth: vec4<f32>,
    jitter: vec2<f32>,
    previous_jitter: vec2<f32>,
    motion_vector_scale: vec2<f32>,
    downscale_factor: vec2<f32>,
    motion_vector_jitter_cancellation: vec2<f32>,
    tan_half_fov: f32,
    jitter_phase_count: f32,
    delta_time: f32,
    delta_pre_exposure: f32,
    view_space_to_meters: f32,
    frame_index: f32,
    velocity_factor: f32,
    reactiveness_scale: f32,
    shading_change_scale: f32,
    accumulation_added_per_frame: f32,
    min_disocclusion_accumulation: f32,
    rcas_lobe: f32,
    pad0: f32,
    pad1: f32,
}

@group(0) @binding(0) var<uniform> cb: Constants;

fn render_size() -> vec2<i32> {
    return cb.render_size;
}

fn upscale_size() -> vec2<i32> {
    return cb.upscale_size;
}

// The size of the half-resolution images (shading change, farthest depth mip 1).
fn shading_change_render_size() -> vec2<i32> {
    return vec2<i32>(vec2<f32>(cb.render_size) * 0.5);
}

fn frame_index() -> f32 {
    return cb.frame_index;
}

fn reconstructed_depth_mv_px_threshold(nearest_depth_in_meters: f32) -> f32 {
    return mix(0.25, 0.75, saturate(nearest_depth_in_meters / 100.0));
}

// View-space distance of a depth buffer value; the factors are described in constants.rs.
fn get_view_space_depth(device_depth: f32) -> f32 {
    return cb.device_to_view_depth.y / (device_depth - cb.device_to_view_depth.x);
}

fn get_view_space_depth_in_meters(device_depth: f32) -> f32 {
    return get_view_space_depth(device_depth) * cb.view_space_to_meters;
}

// Motion in pixels of a 4K screen, the unit the velocity thresholds are tuned in.
fn get_4k_velocity(motion_vector: vec2<f32>) -> f32 {
    return length(motion_vector * vec2<f32>(3840.0, 2160.0));
}

fn ycocg_to_rgb(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(c.x + c.y - c.z, c.x + c.z, c.x - c.y - c.z);
}

fn rgb_to_ycocg(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        0.25 * c.r + 0.5 * c.g + 0.25 * c.b,
        0.5 * c.r - 0.5 * c.b,
        -0.25 * c.r + 0.5 * c.g - 0.25 * c.b,
    );
}

fn rgb_to_luma(linear_rgb: vec3<f32>) -> f32 {
    return dot(linear_rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn tonemap(c: vec3<f32>) -> vec3<f32> {
    return c / (max(max(0.0, c.r), max(c.g, c.b)) + 1.0);
}

fn inverse_tonemap(c: vec3<f32>) -> vec3<f32> {
    return c / max(TONEMAP_EPSILON, 1.0 - max(c.r, max(c.g, c.b)));
}

fn sanitize_color(c: vec3<f32>) -> vec3<f32> {
    return clamp(c, vec3<f32>(0.0), vec3<f32>(COLOR_MAX));
}

fn is_uv_inside(uv: vec2<f32>) -> bool {
    return uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
}

fn is_on_screen(pos: vec2<i32>, size: vec2<i32>) -> bool {
    return all(vec2<u32>(pos) < vec2<u32>(size));
}

fn clamp_load(pos: vec2<i32>, offset: vec2<i32>, size: vec2<i32>) -> vec2<i32> {
    return clamp(pos + offset, vec2<i32>(0), size - vec2<i32>(1));
}

// Moves a uv so that a bilinear fetch stays on the texels of the valid `texture_size` area.
fn clamp_uv(uv: vec2<f32>, texture_size: vec2<i32>, resource_size: vec2<i32>) -> vec2<f32> {
    let location = uv * vec2<f32>(texture_size);
    let clamped = max(vec2<f32>(0.5), min(location, vec2<f32>(texture_size) - vec2<f32>(0.5)));
    return clamped / vec2<f32>(resource_size);
}

fn min_divided_by_max(v0: f32, v1: f32, on_zero: f32) -> f32 {
    let m = max(v0, v1);
    if m != 0.0 {
        return min(v0, v1) / m;
    }
    return on_zero;
}

// The output (high-resolution) pixel that holds the jittered sample of a render pixel.
fn compute_hr_pos_from_lr_pos(lr_pos: vec2<i32>) -> vec2<i32> {
    let jittered = vec2<f32>(lr_pos) + vec2<f32>(0.5) - cb.jitter;
    let in_hr = (jittered / vec2<f32>(cb.render_size)) * vec2<f32>(cb.upscale_size);
    return vec2<i32>(floor(in_hr));
}

struct BilinearTaps {
    base: vec2<i32>,
    fraction: vec2<f32>,
}

// Weights of the four texels (00, 10, 01, 11) of a bilinear fetch at `uv`.
fn get_bilinear_taps(uv: vec2<f32>, size: vec2<i32>) -> BilinearTaps {
    let px = uv * vec2<f32>(size) - vec2<f32>(0.5);
    let base = floor(px);
    return BilinearTaps(vec2<i32>(base), px - base);
}

fn bilinear_weights(fraction: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(
        (1.0 - fraction.x) * (1.0 - fraction.y),
        fraction.x * (1.0 - fraction.y),
        (1.0 - fraction.x) * fraction.y,
        fraction.x * fraction.y,
    );
}

fn tap_offset(index: i32) -> vec2<i32> {
    return vec2<i32>(index & 1, (index >> 1) & 1);
}

// Linear filtering with clamp-to-edge by hand, for one to four channels. `uv` is in the texture's own
// 0..1 range.
fn sample_linear(tex: texture_2d<f32>, uv: vec2<f32>) -> vec4<f32> {
    let size = vec2<i32>(textureDimensions(tex));
    let taps = get_bilinear_taps(uv, size);
    let last = size - vec2<i32>(1);
    let c00 = textureLoad(tex, clamp(taps.base, vec2<i32>(0), last), 0);
    let c10 = textureLoad(tex, clamp(taps.base + vec2<i32>(1, 0), vec2<i32>(0), last), 0);
    let c01 = textureLoad(tex, clamp(taps.base + vec2<i32>(0, 1), vec2<i32>(0), last), 0);
    let c11 = textureLoad(tex, clamp(taps.base + vec2<i32>(1, 1), vec2<i32>(0), last), 0);
    return mix(mix(c00, c10, taps.fraction.x), mix(c01, c11, taps.fraction.x), taps.fraction.y);
}

// A load that returns zero outside the texture, like a DirectX resource load does.
fn load_or_zero(tex: texture_2d<f32>, pos: vec2<i32>) -> vec4<f32> {
    if !is_on_screen(pos, vec2<i32>(textureDimensions(tex))) {
        return vec4<f32>(0.0);
    }
    return textureLoad(tex, pos, 0);
}
