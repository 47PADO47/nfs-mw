// WGSL adaptation of mattatz/THREE.Fire/FireShader.js (MIT), commit pinned in docs/enhanced-exhaust.md.
// Copyright (c) 2015 mattatz
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

struct Globals {
    view_proj: mat4x4<f32>, camera_pos: vec4<f32>, light_dir: vec4<f32>,
    fog_color: vec4<f32>, fog_range: vec4<f32>,
};
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var fire_texture: texture_2d<f32>;
@group(1) @binding(1) var fire_sampler: sampler;
@group(2) @binding(0) var scene_depth: texture_depth_2d;
@group(2) @binding(1) var<uniform> inverse_view_proj: mat4x4<f32>;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) @interpolate(flat) inverse0: vec4<f32>,
    @location(2) @interpolate(flat) inverse1: vec4<f32>,
    @location(3) @interpolate(flat) inverse2: vec4<f32>,
    @location(4) @interpolate(flat) inverse3: vec4<f32>,
    @location(5) @interpolate(flat) parameters: vec4<f32>,
    @location(6) @interpolate(flat) root: vec4<f32>,
    @location(7) @interpolate(flat) tail: vec4<f32>,
    @location(8) @interpolate(flat) shape: vec4<f32>,
    @location(9) @interpolate(flat) root_color: vec4<f32>,
    @location(10) @interpolate(flat) tail_color: vec4<f32>,
    @location(11) @interpolate(flat) contact_plane: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32,
    @location(0) m0: vec4<f32>, @location(1) m1: vec4<f32>,
    @location(2) m2: vec4<f32>, @location(3) m3: vec4<f32>,
    @location(4) i0: vec4<f32>, @location(5) i1: vec4<f32>,
    @location(6) i2: vec4<f32>, @location(7) i3: vec4<f32>, @location(8) parameters: vec4<f32>,
    @location(9) root: vec4<f32>, @location(10) tail: vec4<f32>, @location(11) shape: vec4<f32>,
    @location(12) root_color: vec4<f32>, @location(13) tail_color: vec4<f32>,
    @location(14) contact_plane: vec4<f32>) -> VsOut {
    let corners = array<vec3<f32>, 8>(
        vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>(0.5, -0.5, -0.5),
        vec3<f32>(0.5, 0.5, -0.5), vec3<f32>(-0.5, 0.5, -0.5),
        vec3<f32>(-0.5, -0.5, 0.5), vec3<f32>(0.5, -0.5, 0.5),
        vec3<f32>(0.5, 0.5, 0.5), vec3<f32>(-0.5, 0.5, 0.5));
    let indices = array<u32, 36>(0,2,1,0,3,2, 4,5,6,4,6,7, 0,1,5,0,5,4,
        3,7,6,3,6,2, 0,4,7,0,7,3, 1,2,6,1,6,5);
    var out: VsOut;
    out.world = (mat4x4<f32>(m0,m1,m2,m3) * vec4<f32>(corners[indices[index]], 1.0)).xyz;
    out.clip = globals.view_proj * vec4<f32>(out.world, 1.0);
    out.inverse0 = i0; out.inverse1 = i1; out.inverse2 = i2; out.inverse3 = i3;
    out.parameters = parameters;
    out.root = root; out.tail = tail; out.shape = shape;
    out.root_color = root_color; out.tail_color = tail_color;
    out.contact_plane = contact_plane;
    return out;
}

fn turbulence(p: vec3<f32>) -> f32 {
    var sum = 0.0;
    var frequency = 1.0;
    var amplitude = 1.0;
    for (var octave = 0; octave < 3; octave++) {
        sum += abs(snoise(p * frequency)) * amplitude;
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    return sum;
}

fn sample_fire(local: vec3<f32>, time: f32, seed: f32, blue: f32, swell: f32,
    root: vec4<f32>, tail: vec4<f32>, shape: vec4<f32>, root_color: vec4<f32>, tail_color: vec4<f32>, contact_plane: vec4<f32>) -> vec4<f32> {
    let height = (local.y + 0.5) / swell;
    if height <= 0.0 || height >= 1.0 { return vec4<f32>(0.0); }
    let radial_scale = 3.4 * shape.z;
    // One centreline for all forces. Use unswelled height to match world contact probes.
    let tip = max((local.y + 0.5 - 0.35) / 0.65, 0.0);
    let displacement = shape.xy * tip * tip;
    // A narrow nozzle opens into the turbulent body. Keep the original backfire profile.
    let wall_distance = dot(contact_plane, vec4<f32>(local, 1.0));
    let has_contact = dot(contact_plane.xyz, contact_plane.xyz) > 0.000001;
    let contact = select(0.0, 1.0 - smoothstep(0.02, 0.16, wall_distance), has_contact);
    let wall_fade = select(1.0, smoothstep(0.004, 0.06, wall_distance), has_contact);
    if wall_fade <= 0.0 { return vec4<f32>(0.0); }
    // A confined jet broadens near contact, then fades before entering the surface.
    let radius = mix(1.0, mix(0.38, 1.0, smoothstep(0.0, 0.38, height)), blue)
        * (1.0 + 0.45 * contact * smoothstep(0.1, 0.45, height));
    // The enlarged bend proxy contains mostly empty space. Reject it before
    // evaluating simplex noise, with generous room for both noise components.
    let radial = (local.xz - displacement) * radial_scale;
    let reach = radius + abs(height * 0.7 * tail.w) * 2.0;
    if dot(radial, radial) >= reach * reach { return vec4<f32>(0.0); }
    // Advected eddies bend and separate the tip while the pipe attachment stays fixed.
    let flow = height * 3.5 - time * 1.6;
    let bend = vec2<f32>(snoise(vec3<f32>(flow, seed, 3.1)), snoise(vec3<f32>(seed, flow, 17.7)));
    let offset = bend * height * 0.7 * tail.w - displacement * radial_scale;
    var p = vec3<f32>((local.x * radial_scale + offset.x) / radius, height, (local.z * radial_scale + offset.y) / radius);
    var st = vec2<f32>(length(p.xz), p.y);
    if st.x >= 1.0 || st.y <= 0.0 || st.y >= 1.0 { return vec4<f32>(0.0); }
    p.y -= (seed + time) * 0.55;
    p *= vec3<f32>(mix(1.0, 1.8, blue), 2.0, mix(1.0, 1.8, blue));
    // Compress the reference height lookup so turbulence breaks up a broad tail
    // instead of eroding the whole plume into a short pointed cone.
    st.y = st.y * mix(1.0, 0.7, blue) + sqrt(st.y) * mix(1.3, 0.9, blue) * turbulence(p) * tail.w;
    if st.y >= 1.0 { return vec4<f32>(0.0); }
    // Three.js flips the PNG on upload; the native texture uses top-left image coordinates.
    let source = textureSampleLevel(fire_texture, fire_sampler, vec2<f32>(st.x, 1.0 - st.y), 0.0).rgb;
    // Keep the reference lookup's density and warm edges. Colour follows distance from the pipe,
    // before integration, so a rear view still sees the blue root and hot outer flame together.
    let core = 1.0 - smoothstep(0.05, 0.65, st.x);
    let hot = mix(source, vec3<f32>(source.r) * 2.0, core);
    let warm = source;
    let boundary = 0.018 * bend.x;
    let cold = source.bgr * vec3<f32>(1.0, 1.0, 2.4);
    let white = smoothstep(0.045, 0.18, height + boundary);
    let yellow = smoothstep(0.17, 0.45, height + boundary);
    let gradient = smoothstep(0.06, 0.45, height);
    let tint = mix(root.rgb, tail.rgb, gradient);
    let natural = mix(mix(cold, hot, white), warm, yellow);
    // Optional RGB palettes use emission density rather than the lookup's individual
    // colour channels. Neutral palettes preserve the original reference shading.
    let palette = mix(root_color, tail_color, gradient);
    let recoloured = max(palette.rgb, vec3<f32>(0.0)) * max(natural.r, max(natural.g, natural.b));
    let heated = mix(natural * tint, recoloured, clamp(palette.a, 0.0, 1.0));
    let end_fade = (1.0 - smoothstep(0.85, 1.0, local.y + 0.5))
        * mix(1.0, 1.0 - smoothstep(0.65, 1.0, height), blue);
    var emission = mix(source, heated, blue);
    if shape.w > 0.0 {
        let pulse_palette = mix(root_color.rgb, tail_color.rgb, smoothstep(0.015, 0.12, height));
        let pulse_core = vec3<f32>(1.0, 0.96, 0.82) * 2.0;
        let pulse = mix(pulse_palette, pulse_core, core * smoothstep(0.02, 0.10, height) * 0.9)
            * max(source.r, max(source.g, source.b));
        emission = mix(emission, pulse, shape.w);
    }
    return vec4<f32>(emission * end_fade * root.w, source.r * end_fade * max(blue, shape.w) * root.w) * wall_fade;
}

fn flame(in: VsOut, front: bool) -> vec4<f32> {
    // A reflected marker reverses triangle winding. Keep the same geometric exit faces.
    if front == (in.parameters.w > 0.0) { discard; }
    let inverse = mat4x4<f32>(in.inverse0, in.inverse1, in.inverse2, in.inverse3);
    let origin = (inverse * vec4<f32>(globals.camera_pos.xyz, 1.0)).xyz;
    let back = (inverse * vec4<f32>(in.world, 1.0)).xyz;
    let direction = normalize(back - origin);
    // Avoid 0/0 for axial views. The signs of near-zero components do not affect a ray inside that slab.
    let safe = select(vec3<f32>(1e-6), direction, abs(direction) > vec3<f32>(1e-6));
    let a = (vec3<f32>(-0.5) - origin) / safe;
    let b = (vec3<f32>(0.5) - origin) / safe;
    let lo = min(a, b); let hi = max(a, b);
    let start = max(max(max(lo.x, lo.y), lo.z), 0.0);
    var end = min(min(hi.x, hi.y), hi.z);
    let pixel = vec2<i32>(in.clip.xy);
    let depth = textureLoad(scene_depth, pixel, 0);
    if depth > 0.0 {
        let size = vec2<f32>(textureDimensions(scene_depth));
        let ndc = vec2<f32>(in.clip.x / size.x * 2.0 - 1.0, 1.0 - in.clip.y / size.y * 2.0);
        let hit = inverse_view_proj * vec4<f32>(ndc, depth, 1.0);
        let local_hit = (inverse * vec4<f32>(hit.xyz / hit.w, 1.0)).xyz;
        end = min(end, dot(local_hit - origin, direction));
    }
    if end <= start { return vec4<f32>(0.0); }
    // Fixed work per covered pixel; optical distance keeps oblique views from gaining brightness.
    let samples = u32(clamp(24.0 * in.shape.z / 1.5, 24.0, 64.0));
    let step_length = (end - start) / f32(samples);
    let optical_step = step_length * length(direction * vec3<f32>(in.shape.z, 1.0, in.shape.z));
    let time = in.parameters.x;
    let seed = in.parameters.y;
    // Continuous, aperiodic pressure variation from the reference simplex noise. No frame RNG.
    let swell = 0.82 + 0.18 * snoise(vec3<f32>(time * 1.7, seed, 5.3))
        + 0.10 * snoise(vec3<f32>(time * 5.9, seed, 11.1));
    let flicker = 0.9 + 0.15 * snoise(vec3<f32>(seed, time * 7.3, 19.1));
    var color = vec3<f32>(0.0);
    var transmission = 1.0;
    for (var sample = 0u; sample < samples; sample++) {
        let p = origin + direction * (start + (f32(sample) + 0.5) * step_length);
        let fire = sample_fire(p, time, seed, in.parameters.z, swell, in.root, in.tail, in.shape, in.root_color, in.tail_color, in.contact_plane);
        color += transmission * fire.rgb * optical_step / 0.05;
        // Beer-Lambert attenuation prevents overlapping hot samples from flattening
        // into a clipped yellow silhouette, especially when viewed from behind.
        transmission *= exp(-fire.a * optical_step * 8.0);
    }
    color *= abs(in.parameters.w) * flicker;
    let distance = length(in.world - globals.camera_pos.xyz);
    let fog = clamp((distance - globals.fog_range.x) / max(globals.fog_range.y - globals.fog_range.x, 0.001), 0.0, 1.0);
    return vec4<f32>(color * (1.0 - fog), 0.0);
}
@fragment fn fs_main(in: VsOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> { return flame(in, front); }
@fragment fn fs_main_srgb(in: VsOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let c = flame(in, front);
    let rgb = select(pow((c.rgb + 0.055) / 1.055, vec3<f32>(2.4)), c.rgb / 12.92, c.rgb <= vec3<f32>(0.04045));
    return vec4<f32>(rgb, c.a);
}
