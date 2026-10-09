// FSR 3.1 upscaler pass 8, RCAS: robust contrast-adaptive sharpening of the accumulated image, with
// the noise-removal option AMD enables for the upscaler. It runs on the exposed image (the history
// times the exposure) so that the limits that assume a 0..1 range hold.
//
// Ported to WGSL from AMD's ffx_fsr3upscaler_rcas.h and the RCAS of ffx_fsr1.h (FidelityFX SDK v1.1.4,
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
// Changes from the original: one pixel per thread without the quad remapping, exact reciprocals with
// the divisors kept away from zero (the original relies on its approximations being finite at zero),
// and the neighbours at the edge repeat the edge pixel. `OUTPUT_FORMAT` is replaced with the format
// of the output texture when the module is built.

@group(0) @binding(1) var upscaled_history: texture_2d<f32>;
@group(0) @binding(2) var<storage, read> frame_info: array<vec4<f32>>;
@group(0) @binding(3) var out_upscaled: texture_storage_2d<OUTPUT_FORMAT, write>;

// This is set at the limit of providing unnatural results for sharpening.
const RCAS_LIMIT: f32 = 0.25 - (1.0 / 16.0);
const RCAS_EPSILON: f32 = 1.0 / 65536.0;

fn load_exposed(pos: vec2<i32>) -> vec3<f32> {
    let size = vec2<i32>(textureDimensions(upscaled_history));
    return textureLoad(upscaled_history, clamp(pos, vec2<i32>(0), size - vec2<i32>(1)), 0).rgb * frame_info[0].x;
}

// Simplest multi-channel approximate luma possible (luma times 2).
fn luma2(c: vec3<f32>) -> f32 {
    return c.b * 0.5 + (c.r * 0.5 + c.g);
}

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pos = vec2<i32>(id.xy);
    if !is_on_screen(pos, cb.upscale_size) {
        return;
    }
    // A minimal 3x3 neighbourhood:
    //    b
    //  d e f
    //    h
    let b = load_exposed(pos + vec2<i32>(0, -1));
    let d = load_exposed(pos + vec2<i32>(-1, 0));
    let e = load_exposed(pos);
    let f = load_exposed(pos + vec2<i32>(1, 0));
    let h = load_exposed(pos + vec2<i32>(0, 1));

    // Noise detection: a pixel that differs from the mean of its neighbours by much less than the local
    // contrast range is probably noise and is sharpened less.
    let b_l = luma2(b);
    let d_l = luma2(d);
    let e_l = luma2(e);
    let f_l = luma2(f);
    let h_l = luma2(h);
    var nz = 0.25 * b_l + 0.25 * d_l + 0.25 * f_l + 0.25 * h_l - e_l;
    let luma_range = max(max(max(b_l, d_l), max(e_l, f_l)), h_l) - min(min(min(b_l, d_l), min(e_l, f_l)), h_l);
    nz = saturate(abs(nz) / max(luma_range, RCAS_EPSILON));
    nz = -0.5 * nz + 1.0;

    // Min and max of the ring.
    let mn4 = min(min(b, d), min(f, h));
    let mx4 = max(max(b, d), max(f, h));
    // The limiters solve for the largest lobe that does not clip.
    let hit_min = mn4 / (4.0 * max(mx4, vec3<f32>(RCAS_EPSILON)));
    let hit_max = (vec3<f32>(1.0) - mx4) / min(4.0 * mn4 - vec3<f32>(4.0), vec3<f32>(-RCAS_EPSILON));
    let lobe_rgb = max(-hit_min, hit_max);
    var lobe = max(-RCAS_LIMIT, min(max(lobe_rgb.r, max(lobe_rgb.g, lobe_rgb.b)), 0.0)) * cb.rcas_lobe;
    lobe *= nz;

    // Resolve.
    let rgb = (lobe * b + lobe * d + lobe * h + lobe * f + e) / (4.0 * lobe + 1.0);
    let result = clamp(rgb / frame_info[0].x, vec3<f32>(0.0), vec3<f32>(FP16_MAX));
    textureStore(out_upscaled, pos, vec4<f32>(result, 1.0));
}
