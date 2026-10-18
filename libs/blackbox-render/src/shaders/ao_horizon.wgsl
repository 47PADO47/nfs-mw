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
// https://github.com/GameTechDev/XeGTAO/blob/master/Source/Rendering/Shaders/XeGTAO.hlsli
// This is a compact depth-only adaptation, not the complete XeGTAO implementation.

@group(0) @binding(2) var output_ao: texture_storage_2d<rgba16float, write>;
@group(0) @binding(5) var depth_pyramid: texture_2d<f32>;
const PI: f32 = 3.14159265359;
const HALF_PI: f32 = 1.57079632679;

// XeGTAO's Hilbert/R2 sequence, fixed in time. A 64x64 neighbourhood disperses
// the samples without the former visibly repeating 4x4 ordered strata.
// See vaGTAO.hlsl::SpatioTemporalNoise; the MIT notice above also covers this adaptation.
fn sample_phase(pixel: vec2<u32>) -> vec2<f32> {
    var p = pixel & vec2<u32>(63u);
    var index = 0u;
    for (var level = 32u; level > 0u; level >>= 1u) {
        let rx = select(0u,1u,(p.x & level) != 0u);
        let ry = select(0u,1u,(p.y & level) != 0u);
        index += level * level * ((3u * rx) ^ ry);
        if ry == 0u {
            if rx != 0u { p = vec2<u32>(63u) - p; }
            p = p.yx;
        }
    }
    return fract(vec2<f32>(0.5) + f32(index) * vec2<f32>(0.75487766625,0.56984029100));
}

fn horizon_position(pixel: vec2<i32>, offset: vec2<i32>) -> vec3<f32> {
    if !inside(pixel) { return vec3<f32>(0.0); }
    if params.projection.w < 0.5 { return position_at(pixel); }
    let footprint = max(0.0, floor(log2(max(length(vec2<f32>(offset)),1.0))) - 3.0);
    if footprint < 1.0 { return position_at(pixel); }
    let level = min(u32(footprint)-1u, textureNumLevels(depth_pyramid)-1u);
    let size = textureDimensions(depth_pyramid, level);
    let coordinate = min(vec2<u32>((vec2<f32>(pixel)+0.5)/params.sizes.xy*vec2<f32>(size)),size-vec2<u32>(1));
    let z = textureLoad(depth_pyramid, vec2<i32>(coordinate), level).r;
    if z <= 0.0 { return vec3<f32>(0.0); }
    let center_pixel = (vec2<f32>(coordinate)+0.5)*params.sizes.xy/vec2<f32>(size)-0.5;
    return view_at(center_pixel,z);
}
fn horizon(pixel: vec2<i32>, offset: vec2<i32>, center: vec3<f32>, normal: vec3<f32>, view: vec3<f32>, low: f32) -> f32 {
    let sample = horizon_position(pixel,offset);
    if sample.z == 0.0 { return low; }
    let delta = sample - center;
    let distance = length(delta);
    if distance < 0.015 || dot(delta, normal) <= 0.015 { return low; }
    // Conservative thickness/range heuristic bounds false occlusion from thin, distant objects.
    let range = length(vec3<f32>(delta.xy, delta.z * 1.3));
    let weight = 1.0 - smoothstep(params.effect.x * 0.35, params.effect.x, range);
    return mix(low, dot(delta / distance, view), weight);
}
fn arc(h: f32, n: f32) -> f32 {
    return (cos(n) + 2.0 * h * sin(n) - cos(2.0 * h - n)) * 0.25;
}

@compute @workgroup_size(8, 8)
fn generate(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= vec2<u32>(params.sizes.zw)) { return; }
    let pixel = ao_pixel(vec2<i32>(id.xy));
    let center = position_at(pixel);
    if center.z == 0.0 || -center.z >= params.effect.w {
        textureStore(output_ao, id.xy, vec4<f32>(1.0, 0.0, 0.0, 0.0));
        return;
    }
    let normal = normal_at(pixel, center);
    let view = normalize(-center);
    // Limit angular reach, not a fixed pixel count: 4K must retain the 1080p world-space radius.
    let screen_radius = min(params.effect.x * params.sizes.y / (2.0 * params.projection.y * -center.z), params.sizes.y * (96.0 / 1080.0));
    var visibility = 0.0;
    var open_visibility = 0.0;
    let slices = f32(params.sampling.y);
    let steps = f32(params.sampling.z);
    let phase = sample_phase(id.xy);
    for (var slice = 0u; slice < params.sampling.y; slice++) {
        let phi = (f32(slice) + phase.x) * PI / slices;
        let direction = vec3<f32>(cos(phi), sin(phi), 0.0);
        let ortho = direction - dot(direction, view) * view;
        let axis = normalize(cross(ortho, view));
        let projected = normal - axis * dot(normal, axis);
        let projected_length = max(length(projected), 1e-4);
        let cos_n = clamp(dot(projected, view) / projected_length, 0.0, 1.0);
        let n = sign(dot(ortho, projected)) * acos(cos_n);
        let low0 = cos(n + HALF_PI);
        let low1 = cos(n - HALF_PI);
        var h0 = low0;
        var h1 = low1;
        for (var step = 0u; step < params.sampling.z; step++) {
            let radial_phase = fract(phase.y + f32(slice + step * params.sampling.z) * 0.61803398875);
            let t = (f32(step) + radial_phase) / steps;
            let offset = vec2<i32>(round(vec2<f32>(direction.x, -direction.y) * max(1.5, t * t * screen_radius)));
            h0 = max(h0, horizon(pixel + offset, offset, center, normal, view, low0));
            h1 = max(h1, horizon(pixel - offset, offset, center, normal, view, low1));
        }
        let a0 = n + clamp(-acos(clamp(h1, -1.0, 1.0)) - n, -HALF_PI, HALF_PI);
        let a1 = n + clamp(acos(clamp(h0, -1.0, 1.0)) - n, -HALF_PI, HALF_PI);
        visibility += projected_length * (arc(a0, n) + arc(a1, n));
        // Normalize against the unobstructed slice: open slopes stay bright without a normal buffer.
        open_visibility += projected_length * (arc(n - HALF_PI, n) + arc(n + HALF_PI, n));
    }
    let ao = clamp(visibility / max(open_visibility, 1e-4), 0.0, 1.0);
    let fade = smoothstep(params.effect.z, params.effect.w, -center.z);
    textureStore(output_ao, id.xy, vec4<f32>(mix(ao, 1.0, fade), -center.z, encode_normal(normal)));
}
