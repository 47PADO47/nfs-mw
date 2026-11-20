// Screen-space sun shadows: every thread of a 64-wide wavefront walks one ray towards the light, and the
// wavefront shares its depth reads, so each pixel tests 60 samples for four depth reads.
//
// Ported to WGSL from `WriteScreenSpaceShadow` and `ComputeWavefrontExtents` in Bend Studio's
// `bend_sss_gpu.h` (2023 release). Changed: WGSL, fixed options (no early-out, no debug views, shadow depths
// biased by the bilinear neighbour), reverse-Z depth with far at zero, bounds-checked loads in place of a
// border sampler, and the result faded by the receiver's authored normal as contact.wgsl does.
//
// Copyright 2023 Sony Interactive Entertainment.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// This file has been modified from the original.

struct Params {
    light: vec4<f32>, // pixel coordinate, depth, sign of clip-space w
    sun: vec4<f32>, // unit direction to the sun, strength
    wave_offset: vec2<i32>,
    size: vec2<i32>,
};
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var depth: texture_depth_2d;
@group(0) @binding(2) var normals: texture_2d<f32>;
@group(0) @binding(3) var output: texture_storage_2d<rgba8unorm, write>;

const WAVE: i32 = 64;
// Shadow samples per pixel: the shadow's length in pixels.
const SAMPLES: i32 = 60;
// The nearest samples, any one of which shadows the pixel fully; the others average in four groups.
const HARD_SAMPLES: i32 = 4;
// The farthest samples, which fade the end of the shadow.
const FADE_SAMPLES: i32 = 8;
// Depth reads per thread along its ray: SAMPLES / WAVE + 2.
const READS: i32 = 2;
// A pixel's assumed thickness, as a share of its depth's distance from the far value.
const THICKNESS: f32 = 0.005;
// The depth difference, as the same share, beyond which two neighbours are an edge and are not interpolated.
const BILINEAR_THRESHOLD: f32 = 0.02;
const CONTRAST: f32 = 4.0;

var<workgroup> depth_data: array<f32, 128>;

// Far depth outside the image, as the original's border sampler returns.
fn depth_at(pixel: vec2<i32>) -> f32 {
    if any(pixel < vec2<i32>(0)) || any(pixel >= params.size) { return 0.0; }
    return textureLoad(depth, pixel, 0);
}

@compute @workgroup_size(64, 1, 1)
fn shadow(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_id) local: vec3<u32>) {
    let thread = i32(local.x);
    // The wavefront's start pixel, and the step to the pixel WAVE pixels along the ray.
    var xy = vec2<i32>(group.yz) * WAVE + params.wave_offset;
    let light_xy = floor(params.light.xy) + 0.5;
    let light_fraction = params.light.xy - light_xy;
    let reverse = params.light.w > 0.0;
    let sign_xy = sign(xy);
    let horizontal = abs(xy.x + sign_xy.y) < abs(xy.y - sign_xy.x);
    let axis = vec2<i32>(select(0, sign_xy.y, horizontal), select(-sign_xy.x, 0, horizontal));
    xy = axis * i32(group.x) + xy;
    let xy_f = vec2<f32>(xy);
    // Interpolation towards the light only follows the larger axis.
    let x_major = abs(xy_f.x) > abs(xy_f.y);
    let major = select(xy_f.y, xy_f.x, x_major);
    let major_start = abs(major);
    let major_end = abs(major) - f32(WAVE);
    var light_major = select(light_fraction.y, light_fraction.x, x_major);
    light_major = select(light_major, -light_major, major > 0.0);
    let start_xy = xy_f + light_xy;
    // The innermost ring interpolates to a pixel-centred coordinate, so rounding skips no output pixel.
    let end_xy = mix(params.light.xy, start_xy, (major_end + light_major) / (major_start + light_major));
    let xy_delta = start_xy - end_xy;
    let thread_step = f32(thread ^ select(WAVE - 1, 0, reverse));
    var pixel_xy = mix(start_xy, end_xy, thread_step / f32(WAVE));
    let pixel_distance = major_start - thread_step + light_major;

    let direction = -params.light.w;
    // Near depth is larger than far depth.
    let z_sign = -1.0;
    let write_xy = vec2<i32>(floor(pixel_xy));
    var sampling_depth: array<f32, 2>;
    var shadowing_depth: array<f32, 2>;
    var thickness_scale: array<f32, 2>;
    var sample_distance: array<f32, 2>;
    for (var i = 0; i < READS; i++) {
        // Two depths per read, interpolated on the ray's minor axis unless they form an edge.
        let read_xy = vec2<i32>(floor(pixel_xy));
        let minor = select(pixel_xy.x, pixel_xy.y, x_major);
        let bias = select(-1, 1, fract(minor) - 0.5 > 0.0);
        let offset = vec2<i32>(select(bias, 0, x_major), select(0, bias, x_major));
        let near = depth_at(read_xy);
        let beside = depth_at(read_xy + offset);
        thickness_scale[i] = abs(near);
        let edge = abs(near - beside) > thickness_scale[i] * BILINEAR_THRESHOLD;
        sampling_depth[i] = near;
        // Any sample of this wavefront may lie towards the neighbour, so it shadows from the farther depth.
        shadowing_depth[i] = select(near + abs(near - beside) * z_sign, near, edge);
        sample_distance[i] = pixel_distance + f32(WAVE * i) * direction;
        pixel_xy += xy_delta * direction;
    }
    for (var i = 0; i < READS; i++) {
        // Perspective-corrected: in this space all light rays are parallel.
        var stored = (shadowing_depth[i] - params.light.z) / sample_distance[i];
        // An extended read near the light may overshoot it.
        if i != 0 && sample_distance[i] <= 0.0 { stored = 1e10; }
        depth_data[i * WAVE + thread] = stored;
    }
    workgroupBarrier();

    if any(write_xy < vec2<i32>(0)) || any(write_xy >= params.size) { return; }
    let lit = vec4<f32>(1.0);
    if sampling_depth[0] <= 0.0 { textureStore(output, write_xy, lit); return; }
    let encoded = textureLoad(normals, write_xy, 0);
    if encoded.a < 0.5 { textureStore(output, write_xy, lit); return; }
    let facing = dot(normalize(encoded.xyz * 2.0 - 1.0), params.sun.xyz);
    if facing <= 0.08 { textureStore(output, write_xy, lit); return; }

    // The inverse width of the shadowing window for the projected depths; see the original for its terms.
    let depth_scale = min(sample_distance[0] + direction, 1.0 / THICKNESS) * sample_distance[0] / thickness_scale[0];
    let start_depth = (sampling_depth[0] - params.light.z) / sample_distance[0] * depth_scale - z_sign;
    let first = thread + 1;
    var hard = 1.0;
    var soft = vec4<f32>(1.0);
    for (var i = 0; i < HARD_SAMPLES; i++) {
        hard = min(hard, abs(start_depth - depth_data[first + i] * depth_scale));
    }
    for (var i = HARD_SAMPLES; i < SAMPLES - FADE_SAMPLES; i++) {
        soft[i & 3] = min(soft[i & 3], abs(start_depth - depth_data[first + i] * depth_scale));
    }
    for (var i = SAMPLES - FADE_SAMPLES; i < SAMPLES; i++) {
        let fade = f32(i + 1 - (SAMPLES - FADE_SAMPLES)) / f32(FADE_SAMPLES + 1) * 0.75;
        soft[i & 3] = min(soft[i & 3], abs(start_depth - depth_data[first + i] * depth_scale) + fade);
    }
    // Zero is a sample exactly at the reference depth; the contrast lets near matches shadow fully too.
    soft = clamp(soft * CONTRAST + (1.0 - CONTRAST), vec4<f32>(0.0), vec4<f32>(1.0));
    hard = clamp(hard * CONTRAST + (1.0 - CONTRAST), 0.0, 1.0);
    let visibility = min(hard, dot(soft, vec4<f32>(0.25)));
    let shadow = (1.0 - visibility) * params.sun.w * smoothstep(0.08, 0.3, facing);
    textureStore(output, write_xy, vec4<f32>(vec3<f32>(1.0 - shadow), 1.0));
}
