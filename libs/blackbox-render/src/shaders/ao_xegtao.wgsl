// XeGTAO spatial main pass, adapted to WGSL / reverse-Z / right-handed view space.
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
// Source: GameTechDev/XeGTAO, a5b1686c7ea37788eeb3576b5be47f7c03db532c,
// Source/Rendering/Shaders/XeGTAO.hlsli: MainPass, CalculateEdges, PackEdges.
// Shares the licensed Hilbert/R2 noise and arc integral from ao_horizon.wgsl.

fn xe_edges(pixel: vec2<i32>, center_z: f32) -> f32 {
    let step=i32(params.sampling.x);
    var edges = vec4<f32>(
        -position_at(pixel + vec2<i32>(-step,0)).z,
        -position_at(pixel + vec2<i32>(step,0)).z,
        -position_at(pixel + vec2<i32>(0,-step)).z,
        -position_at(pixel + vec2<i32>(0,step)).z) - center_z;
    let slope = vec2<f32>(edges.y-edges.x, edges.w-edges.z) * 0.5;
    edges = min(abs(edges), abs(edges + vec4<f32>(slope.x,-slope.x,slope.y,-slope.y)));
    let continuity = clamp(1.25 - edges / max(center_z * 0.011, 1e-6), vec4<f32>(0.0), vec4<f32>(1.0));
    return dot(round(continuity * 2.9), vec4<f32>(64.0,16.0,4.0,1.0));
}

fn xe_position(pixel: vec2<i32>, offset: vec2<i32>) -> vec3<f32> {
    if !inside(pixel) { return vec3<f32>(0.0); }
    if params.projection.w < 0.5 { return position_at(pixel); }
    let mip = max(0.0, round(log2(max(length(vec2<f32>(offset)),1.0)) - 3.3));
    if mip < 1.0 { return position_at(pixel); }
    let level = min(u32(mip)-1u, textureNumLevels(depth_pyramid)-1u);
    let size = textureDimensions(depth_pyramid, level);
    let coordinate = min(vec2<u32>((vec2<f32>(pixel)+0.5)/params.sizes.xy*vec2<f32>(size)),size-vec2<u32>(1));
    let z = textureLoad(depth_pyramid, vec2<i32>(coordinate), level).r;
    if z <= 0.0 { return vec3<f32>(0.0); }
    // Reconstruct at the selected point sample's center, avoiding false slopes at coarse LODs.
    let center_pixel = (vec2<f32>(coordinate)+0.5)*params.sizes.xy/vec2<f32>(size)-0.5;
    return view_at(center_pixel,z);
}

fn xe_horizon(pixel: vec2<i32>, offset: vec2<i32>, center: vec3<f32>, normal: vec3<f32>, view: vec3<f32>, low: f32) -> f32 {
    let sample = xe_position(pixel,offset);
    if sample.z == 0.0 { return low; }
    let delta = sample-center;
    let distance = length(delta);
    if distance < 1e-6 { return low; }
    // Rounded offsets can leave the analytic slice; coplanar taps are not blockers.
    if dot(delta,normal) <= 0.005 { return low; }
    let radius = params.effect.x * 1.457;
    let weight = clamp((radius-distance)/(0.615*radius),0.0,1.0);
    return mix(low,dot(delta/distance,view),weight);
}

@compute @workgroup_size(8,8)
fn generate_xegtao(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= vec2<u32>(params.sizes.zw)) { return; }
    let pixel = ao_pixel(vec2<i32>(id.xy));
    let position = position_at(pixel);
    if position.z == 0.0 || -position.z >= params.effect.w {
        textureStore(output_ao,id.xy,vec4<f32>(1.0,0.0,0.0,0.0));
        return;
    }
    let normal = normal_at(pixel,position);
    // FP32 reference bias towards the eye; no history, frame noise or bent normals.
    let center = position * 0.99999;
    let view = normalize(-center);
    let radius = params.effect.x * 1.457;
    let screen_radius = radius * params.sizes.x / (2.0 * params.projection.x * -center.z);
    let min_s = 1.3 / screen_radius;
    let slices = f32(params.sampling.y);
    let steps = f32(params.sampling.z);
    let noise = sample_phase(id.xy);
    var visibility = clamp((10.0-screen_radius)/100.0,0.0,1.0)*0.5;
    var open_visibility = 0.0;
    for (var slice=0u; slice<params.sampling.y; slice++) {
        let phi = (f32(slice)+noise.x)*PI/slices;
        let direction = vec3<f32>(cos(phi),sin(phi),0.0);
        let ortho = direction-dot(direction,view)*view;
        let axis = safe_normal(cross(ortho,view),vec3<f32>(1.0,0.0,0.0));
        let projected = normal-axis*dot(normal,axis);
        let projected_length = max(length(projected),1e-4);
        let cos_n = clamp(dot(projected,view)/projected_length,0.0,1.0);
        let n = sign(dot(ortho,projected))*acos(cos_n);
        let low0 = cos(n+HALF_PI);
        let low1 = cos(n-HALF_PI);
        var h0=low0;
        var h1=low1;
        for (var step=0u; step<params.sampling.z; step++) {
            let phase = fract(noise.y+f32(slice+step*params.sampling.z)*0.61803398875);
            let s = (f32(step)+phase)/steps;
            let offset = vec2<i32>(round(vec2<f32>(direction.x,-direction.y)*(s*s+min_s)*screen_radius));
            h0=max(h0,xe_horizon(pixel+offset,offset,center,normal,view,low0));
            h1=max(h1,xe_horizon(pixel-offset,offset,center,normal,view,low1));
        }
        let a0=-acos(clamp(h1,-1.0,1.0));
        let a1=acos(clamp(h0,-1.0,1.0));
        visibility += mix(projected_length,1.0,0.05)*(arc(a0,n)+arc(a1,n));
        // Normalize discrete slice quadrature against the same unobstructed slope.
        open_visibility += mix(projected_length,1.0,0.05)*(arc(n-HALF_PI,n)+arc(n+HALF_PI,n));
    }
    let ao=max(0.03,pow(clamp(visibility/max(open_visibility,1e-4),0.0,1.0),2.2));
    let fade=smoothstep(params.effect.z,params.effect.w,-position.z);
    // Keep reference edge bytes exactly representable in FP16, alongside unscaled visibility.
    textureStore(output_ao,id.xy,vec4<f32>(mix(ao,1.0,fade),-position.z,xe_edges(pixel,-position.z),0.0));
}
