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
// Source/Rendering/Shaders/XeGTAO.hlsli: MainPass (with its thin occluder compensation), CalculateEdges, PackEdges, optional directional moment.
// Shares the licensed arc integral from ao_horizon.wgsl.

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

// lowbias32, Hash Function Prospector (Chris Wellons), Unlicense/public domain.
// https://github.com/skeeto/hash-prospector/tree/396dbe235c94dfc2e9b559fc965bcfda8b6a122c
// README.md, lowbias32.
// Unlike the correlated Hilbert/R2 pair, these independent phases do not form
// visible columns in sparse horizon searches without a following TAA pass.
fn xe_hash(value: u32) -> u32 {
    var h = value;
    h = (h ^ (h >> 16u)) * 0x7feb352du;
    h = (h ^ (h >> 15u)) * 0x846ca68bu;
    return h ^ (h >> 16u);
}

fn xe_phase(pixel: vec2<u32>) -> vec2<f32> {
    let seed = xe_hash(pixel.x ^ xe_hash(pixel.y) ^ (u32(params.fog.w) * 0x9e3779b9u));
    let phase = vec2<f32>(f32(seed & 0xffffffu), f32(xe_hash(seed) & 0xffffffu)) / 16777216.0;
    // Under a temporal upscaler each pixel's slice angle steps by the golden ratio per frame and its radial
    // phase by the plastic number's, so the frames the upscaler accumulates cover both evenly.
    return fract(phase + params.fog.x * vec2<f32>(0.61803398875, 0.75487766625));
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

// The depth of the static scene alone, copied in the prepass before the dynamic models (cars, characters) were drawn,
// and a mip chain of 16-pixel screen tiles that are 1 where a dynamic model covers it. Only read while
// params.tuning.w is set; otherwise 1x1 placeholders are bound.
@group(0) @binding(6) var static_depth: texture_2d<f32>;
@group(0) @binding(7) var dynamic_tiles: texture_2d<f32>;
const XE_TILE: f32 = 16.0;

// Whether a dynamic model covers any tile within `reach` pixels: the 3x3 tiles around the pixel at the finest level
// whose tiles are at least `reach` wide.
fn xe_dynamic_near(pixel: vec2<i32>, reach: f32) -> bool {
    let top = textureNumLevels(dynamic_tiles)-1u;
    let level = u32(clamp(ceil(log2(max(reach,1.0)/XE_TILE)),0.0,f32(top)));
    let tile = XE_TILE*exp2(f32(level));
    let size = vec2<i32>(textureDimensions(dynamic_tiles,level));
    let center = vec2<i32>(vec2<f32>(pixel)/tile);
    for (var y=-1; y<=1; y++) {
        for (var x=-1; x<=1; x++) {
            let q = center+vec2<i32>(x,y);
            if any(q<vec2<i32>(0)) || any(q>=size) { continue; }
            if textureLoad(dynamic_tiles,q,level).r > 0.5 { return true; }
        }
    }
    // Below the coarsest level the 3x3 tiles span at least `reach` each way; the coarsest is one tile over the image.
    return false;
}

// Whether a dynamic model covers the tap's 16-pixel tile.
fn xe_dynamic_tile(pixel: vec2<i32>) -> bool {
    if !inside(pixel) { return false; }
    return textureLoad(dynamic_tiles,vec2<i32>(vec2<f32>(pixel)/XE_TILE),0).r > 0.5;
}

// Whether the receiver is a static surface near a dynamic model: no model is drawn over it, one is within reach.
fn xe_static_receiver(pixel: vec2<i32>, reach: f32) -> bool {
    if params.tuning.w < 0.5 || !xe_dynamic_near(pixel,reach) { return false; }
    return textureLoad(static_depth,pixel,0).r == depth_at(pixel);
}

// The static surface a dynamic model hides at this tap, or zero where the tap sees the static scene itself.
fn xe_hidden_static(pixel: vec2<i32>) -> vec3<f32> {
    let full = depth_at(pixel);
    if full <= 0.0 { return vec3<f32>(0.0); }
    let d = textureLoad(static_depth,pixel,0).r;
    // Reverse-Z: the model is nearer, so the static surface behind it has the smaller value.
    if d <= 0.0 || d >= full { return vec3<f32>(0.0); }
    return view_at(vec2<f32>(pixel),params.projection.z/d);
}

fn xe_weighted(delta: vec3<f32>, distance: f32, view: vec3<f32>, low: f32) -> f32 {
    let radius = params.effect.x * 1.457;
    // Thin occluder compensation (XeGTAO MainPass falloffBase): the tap's view-space depth offset is stretched
    // before the falloff, so a blocker well in front of the receiver fades out sooner.
    let falloff = length(vec3<f32>(delta.xy, delta.z*(1.0+params.tuning.x)));
    let weight = clamp((radius-falloff)/(0.615*radius),0.0,1.0);
    return mix(low,dot(delta/distance,view),weight);
}

// A static receiver's horizon also counts the static surface a dynamic model hides at the tap. The screen-space
// search otherwise finds only the model, which the falloff discounts, so a car parked before a wall's base erased
// the occlusion of the crease beside it.
fn xe_horizon(pixel: vec2<i32>, offset: vec2<i32>, center: vec3<f32>, normal: vec3<f32>, view: vec3<f32>, low: f32,
              receiver_static: bool) -> f32 {
    let visible = xe_visible_horizon(pixel,offset,center,normal,view,low);
    if !receiver_static || !xe_dynamic_tile(pixel) { return visible; }
    let hidden = xe_hidden_static(pixel);
    if hidden.z == 0.0 { return visible; }
    let delta = hidden-center;
    let distance = length(delta);
    if distance < 1e-6 || dot(delta,normal) <= 0.005 { return visible; }
    return max(visible,xe_weighted(delta,distance,view,low));
}

fn xe_visible_horizon(pixel: vec2<i32>, offset: vec2<i32>, center: vec3<f32>, normal: vec3<f32>, view: vec3<f32>,
                      low: f32) -> f32 {
    let sample = xe_position(pixel,offset);
    if sample.z == 0.0 { return low; }
    let delta = sample-center;
    let distance = length(delta);
    if distance < 1e-6 { return low; }
    // Rounded offsets can leave the analytic slice; coplanar taps are not blockers.
    if dot(delta,normal) <= 0.005 { return low; }
    // Filtered hierarchy depths are not exact surface points on perspective slopes.
    // Confirm potential blockers against scene depth, so a coarse footprint cannot
    // invent occlusion on an otherwise open receiver plane.
    if params.projection.w > 0.5 {
        let exact = position_at(pixel);
        if exact.z == 0.0 || dot(exact-center,normal) <= 0.005 { return low; }
    }
    return xe_weighted(delta,distance,view,low);
}

// GTAO Algorithm 2 (via XeGTAO): first moment of the unoccluded arc in
// the slice basis (positive screen direction, direction to the eye).
fn xe_bent_moment(a0: f32, a1: f32, n: f32, sin_n: f32, cos_n: f32) -> vec2<f32> {
    let t0 = 6.0*(sin(a0-n)+sin(a1-n)) - sin(3.0*a0-n) - sin(3.0*a1-n)
        + 16.0*sin_n - 3.0*(sin(a0+n)+sin(a1+n));
    let t1 = -cos(3.0*a0-n) - cos(3.0*a1-n) + 8.0*cos_n - 3.0*(cos(a0+n)+cos(a1+n));
    return vec2(t0,t1)/12.0;
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
    // FP32 reference bias towards the eye.
    let center = position * 0.99999;
    let view = normalize(-center);
    let radius = params.effect.x * 1.457;
    let screen_radius = radius * params.sizes.x / (2.0 * params.projection.x * -center.z);
    let min_s = 1.3 / screen_radius;
    let slices = f32(params.sampling.y);
    let steps = f32(params.sampling.z);
    let noise = xe_phase(id.xy);
    // Taps reach (s*s+min_s)*screen_radius pixels: just over the screen radius.
    let receiver_static = xe_static_receiver(pixel,screen_radius+2.0);
    var visibility = clamp((10.0-screen_radius)/100.0,0.0,1.0)*0.5;
    var open_visibility = 0.0;
    var bent_moment = vec3(0.0);
    var open_moment = vec3(0.0);
    let directional = params.tuning.z > 0.5;
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
            h0=max(h0,xe_horizon(pixel+offset,offset,center,normal,view,low0,receiver_static));
            h1=max(h1,xe_horizon(pixel-offset,offset,center,normal,view,low1,receiver_static));
        }
        let a0=-acos(clamp(h1,-1.0,1.0));
        let a1=acos(clamp(h0,-1.0,1.0));
        visibility += mix(projected_length,1.0,0.05)*(arc(a0,n)+arc(a1,n));
        // Normalize discrete slice quadrature against the same unobstructed slope.
        open_visibility += mix(projected_length,1.0,0.05)*(arc(n-HALF_PI,n)+arc(n+HALF_PI,n));
        if directional {
            let tangent = safe_normal(ortho,direction);
            let sin_n = sin(n);
            let moment = xe_bent_moment(a0,a1,n,sin_n,cos_n);
            bent_moment += (tangent*moment.x+view*moment.y)*projected_length;
            // The open hemisphere has these closed-form moments.
            open_moment += (tangent*(16.0*sin_n)+view*(8.0*cos_n))*(projected_length/12.0);
        }
    }
    let ao=max(0.03,pow(clamp(visibility/max(open_visibility,1e-4),0.0,1.0),2.2));
    let fade=smoothstep(params.effect.z,params.effect.w,-position.z);
    var bent = 0.0;
    if directional {
        // Remove the finite-slice bias so an open surface keeps its original normal.
        let delta = safe_normal(bent_moment,normal)-safe_normal(open_moment,normal);
        let direction = safe_normal(normal+delta*(1.0-fade),normal);
        let world = vec3(dot(params.view[0].xyz,direction),dot(params.view[1].xyz,direction),dot(params.view[2].xyz,direction));
        bent = pack_ao_normal(world);
    }
    // Keep reference edge bytes exactly representable in FP16, alongside unscaled visibility.
    textureStore(output_ao,id.xy,vec4<f32>(mix(ao,1.0,fade),-position.z,xe_edges(pixel,-position.z),bent));
}
