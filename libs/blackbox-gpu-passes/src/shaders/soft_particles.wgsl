// Original procedural mask and reverse-Z depth reconstruction.
// Soft intersection concept: GPU Gems 3 chapter 23.4 (reference only; no code copied).
struct Globals {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    fog_color: vec4<f32>,
    fog_range: vec4<f32>,
};
struct Parameters {
    inverse_view_proj: mat4x4<f32>,
    fade: vec4<f32>,
};
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var scene_depth: texture_depth_2d;
@group(1) @binding(1) var<uniform> parameters: Parameters;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world: vec3<f32>,
    @location(3) detail: vec2<f32>,
};
@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>,
           @location(2) uv: vec2<f32>, @location(3) detail: vec2<f32>) -> VsOut {
    var out: VsOut;
    out.clip = globals.view_proj * vec4<f32>(position, 1.0);
    out.color = color;
    out.uv = uv;
    out.world = position;
    out.detail = detail;
    return out;
}

fn hash(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(0.1031, 0.1099));
    let t = q + dot(q, q.yx + vec2<f32>(19.19));
    return fract(t.x * t.y * 17.0);
}
fn noise(p: vec2<f32>) -> f32 {
    let cell = floor(p);
    let f = fract(p);
    let t = f * f * (vec2<f32>(3.0) - 2.0 * f);
    return mix(mix(hash(cell), hash(cell + vec2<f32>(1.0, 0.0)), t.x),
               mix(hash(cell + vec2<f32>(0.0, 1.0)), hash(cell + vec2<f32>(1.0)), t.x), t.y);
}
fn density(p: vec2<f32>, age: f32, seed: f32) -> f32 {
    let offset = vec2<f32>(seed, -age * 0.7);
    let low = noise(p * 2.4 + offset);
    let mid = noise(p * 5.1 + offset * 1.3 + vec2<f32>(7.0));
    let fine = noise(p * 11.3 + offset * 1.7);
    let radius = dot(p, p);
    let edge = 1.0 - smoothstep(0.08, 0.9 + low * 0.16, radius);
    let detail = smoothstep(0.08, 0.9, low * 0.55 + mid * 0.3 + fine * 0.15);
    return edge * edge * (0.2 + 0.8 * detail);
}
fn intersection(in: VsOut) -> f32 {
    let pixel = vec2<i32>(in.clip.xy);
    let depth = textureLoad(scene_depth, pixel, 0);
    if depth > in.clip.z { return 0.0; }
    if depth <= 0.0 { return 1.0; }
    let size = vec2<f32>(textureDimensions(scene_depth));
    let ndc = vec2<f32>(in.clip.x / size.x * 2.0 - 1.0, 1.0 - in.clip.y / size.y * 2.0);
    let homogeneous = parameters.inverse_view_proj * vec4<f32>(ndc, depth, 1.0);
    let scene_point = homogeneous.xyz / homogeneous.w;
    let scene_distance = length(scene_point - globals.camera_pos.xyz);
    let particle_distance = length(in.world - globals.camera_pos.xyz);
    return smoothstep(0.0, parameters.fade.x, scene_distance - particle_distance);
}
fn particle_color(in: VsOut) -> vec4<f32> {
    let p = in.uv * 2.0 - 1.0;
    let mask = density(p, in.detail.x, in.detail.y);
    let distance = length(in.world - globals.camera_pos.xyz);
    let fog = clamp((distance - globals.fog_range.x) / max(globals.fog_range.y - globals.fog_range.x, 0.001), 0.0, 1.0);
    let shade = 0.88 + 0.12 * noise(p * 3.0 + vec2<f32>(in.detail.y));
    let color = mix(in.color.rgb * shade, globals.fog_color.rgb, fog);
    return vec4<f32>(color, in.color.a * mask * intersection(in));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return particle_color(in);
}

// The exact sRGB decode, so decode then the hardware's encode on write is the identity; see
// `world::is_srgb`.
fn linear_from_gamma(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_main_srgb(in: VsOut) -> @location(0) vec4<f32> {
    let c = particle_color(in);
    return vec4<f32>(linear_from_gamma(c.rgb), c.a);
}
