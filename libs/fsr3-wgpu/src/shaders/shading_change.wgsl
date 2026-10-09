// FSR 3.1 upscaler pass 4, shading change: turns the pyramid of luma differences into a per-pixel
// estimate (0 to 1) of how much the shading of that area changed since the previous frame.
//
// Ported to WGSL from AMD's ffx_fsr3upscaler_shading_change.h (FidelityFX SDK v1.1.4, FSR 3.1.4):
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
// Changes from the original: the three pyramid levels are three textures, filtered by hand.

@group(0) @binding(1) var pyramid0: texture_2d<f32>;
@group(0) @binding(2) var pyramid1: texture_2d<f32>;
@group(0) @binding(3) var pyramid2: texture_2d<f32>;
@group(0) @binding(4) var out_shading_change: texture_storage_2d<r32float, write>;

// |mean difference * mean sign|: large where the whole neighbourhood changed in one direction.
fn level_signal(level: texture_2d<f32>, uv: vec2<f32>) -> f32 {
    let value = sample_linear(level, uv).xy;
    return abs(value.x * value.y);
}

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pos = vec2<i32>(id.xy);
    let size = shading_change_render_size();
    if !is_on_screen(pos, size) {
        return;
    }
    let uv = (vec2<f32>(pos) + vec2<f32>(0.5)) / vec2<f32>(size);
    let uv_jittered = uv + cb.jitter / vec2<f32>(cb.render_size);
    let level_uv = clamp_uv(uv_jittered, size, vec2<i32>(textureDimensions(pyramid0)));

    var shading_change = 0.0;
    let s0 = level_signal(pyramid0, level_uv);
    if s0 > 0.0 {
        shading_change = max(shading_change, s0);
    }
    let s1 = level_signal(pyramid1, level_uv);
    if s1 > 0.0 {
        shading_change = max(shading_change, s1);
    }
    let s2 = level_signal(pyramid2, level_uv);
    if s2 > 0.0 {
        shading_change = max(shading_change, s2);
    }
    textureStore(out_shading_change, pos, vec4<f32>(saturate(shading_change), 0.0, 0.0, 0.0));
}
