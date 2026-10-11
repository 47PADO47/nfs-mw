// FSR 3.1 upscaler pass 2, luma pyramid and auto exposure. `cs_reduce` makes the half-resolution
// farthest depth (mip 1) and sums the log luminance and luminance of the frame per workgroup;
// `cs_final` adds the sums up, smooths the average over time and stores the frame's exposure.
//
// Ported to WGSL from AMD's ffx_fsr3upscaler_luma_pyramid.h and ComputeAutoExposureFromLavg of
// ffx_fsr3upscaler_common.h (FidelityFX SDK v1.1.4, FSR 3.1.4):
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
// Changes from the original: AMD's single-pass downsampler (SPD) needs a cross-workgroup atomic
// counter and globally coherent storage, which WGSL cannot promise, so the reduction takes two
// dispatches. It averages exactly over the pixels of the frame (SPD repeats the edge pixels to fill
// its 64x64 tiles), and the 6-level pyramid of which only the top and the 1x1 level were used is gone.
// The exposure is also taken from the input texture here when auto exposure is off, so that later
// passes read one place: `frame_info[0]` is [exposure, smoothed log luma, average luma, 0].

@group(0) @binding(1) var current_luma: texture_2d<f32>;
@group(0) @binding(2) var farthest_depth: texture_2d<f32>;
@group(0) @binding(3) var out_farthest_depth_mip1: texture_storage_2d<r32float, write>;
@group(0) @binding(4) var<storage, read_write> partials: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read_write> frame_info: array<vec4<f32>>;
@group(0) @binding(6) var input_exposure: texture_2d<f32>;

// The smoothed log luma holds this when the history was reset: the frame's own value is then used.
const RESET_AUTO_EXPOSURE_SMOOTHING: f32 = 1e4;

var<workgroup> shared_sums: array<vec4<f32>, 256>;

// The log luma, the luma and the pixel count of one pixel, or zeros outside the frame.
fn pixel_sums(pos: vec2<i32>) -> vec3<f32> {
    if !is_on_screen(pos, cb.render_size) {
        return vec3<f32>(0.0);
    }
    let luma = textureLoad(current_luma, pos, 0).x;
    return vec3<f32>(max(EPSILON, log(luma)), luma, 1.0);
}

@compute @workgroup_size(8, 8, 1)
fn cs_reduce(
    @builtin(global_invocation_id) id: vec3<u32>,
    @builtin(workgroup_id) group: vec3<u32>,
    @builtin(num_workgroups) groups: vec3<u32>,
    @builtin(local_invocation_index) index: u32,
) {
    // A thread owns the 2x2 block that makes one texel of the half-resolution images.
    let texel = vec2<i32>(id.xy);
    let origin = texel * 2;
    var sums = pixel_sums(origin);
    sums += pixel_sums(origin + vec2<i32>(1, 0));
    sums += pixel_sums(origin + vec2<i32>(0, 1));
    sums += pixel_sums(origin + vec2<i32>(1, 1));

    // The farthest depth keeps its half-resolution mean (the blocks of the last odd row and column
    // have no texel).
    if all(texel < vec2<i32>(textureDimensions(out_farthest_depth_mip1))) {
        let sum = textureLoad(farthest_depth, origin, 0).x
            + textureLoad(farthest_depth, origin + vec2<i32>(1, 0), 0).x
            + textureLoad(farthest_depth, origin + vec2<i32>(0, 1), 0).x
            + textureLoad(farthest_depth, origin + vec2<i32>(1, 1), 0).x;
        textureStore(out_farthest_depth_mip1, texel, vec4<f32>(sum * 0.25, 0.0, 0.0, 0.0));
    }

    // Add up the workgroup. Every thread takes part in the barriers.
    shared_sums[index] = vec4<f32>(sums, 0.0);
    workgroupBarrier();
    for (var stride = 32u; stride > 0u; stride = stride >> 1u) {
        if index < stride {
            shared_sums[index] += shared_sums[index + stride];
        }
        workgroupBarrier();
    }
    if index == 0u {
        partials[group.y * groups.x + group.x] = shared_sums[0];
    }
}

fn compute_auto_exposure_from_lavg(log_average: f32) -> f32 {
    let average = exp(log_average);
    let s = 100.0;
    let k = 12.5;
    let exposure_iso100 = log2((average * s) / k);
    let q = 0.65;
    let l_max = (78.0 / (q * s)) * pow(2.0, exposure_iso100);
    return 1.0 / l_max;
}

@compute @workgroup_size(256, 1, 1)
fn cs_final(@builtin(local_invocation_index) index: u32) {
    let half_size = (cb.render_size + vec2<i32>(1)) / 2;
    let count = u32(((half_size.x + 7) / 8) * ((half_size.y + 7) / 8));
    var sums = vec3<f32>(0.0);
    for (var i = index; i < count; i += 256u) {
        sums += partials[i].xyz;
    }
    shared_sums[index] = vec4<f32>(sums, 0.0);
    workgroupBarrier();
    for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
        if index < stride {
            shared_sums[index] += shared_sums[index + stride];
        }
        workgroupBarrier();
    }
    if index != 0u {
        return;
    }

    let total = shared_sums[0];
    let pixels = max(total.z, 1.0);
    var info = frame_info[0];
    let previous_log_luma = info.y;
    var log_luma = total.x / pixels;
    if previous_log_luma < RESET_AUTO_EXPOSURE_SMOOTHING {
        log_luma = previous_log_luma + (log_luma - previous_log_luma) * (1.0 - exp(-cb.delta_time));
        log_luma = max(0.0, log_luma);
    }

    var exposure = compute_auto_exposure_from_lavg(log_luma);
    if !AUTO_EXPOSURE {
        exposure = textureLoad(input_exposure, vec2<i32>(0, 0), 0).x;
        if !(exposure > 0.0) {
            exposure = 1.0;
        }
    }
    info.x = exposure;
    info.y = log_luma;
    info.z = total.y / pixels;
    frame_info[0] = info;
}
