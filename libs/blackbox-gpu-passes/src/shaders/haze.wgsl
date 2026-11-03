// An inexpensive bounded light-scattering approximation. Eight samples are clipped to scene depth.
struct Globals {
    view_proj: mat4x4<f32>, camera_pos: vec4<f32>, light_dir: vec4<f32>,
    fog_color: vec4<f32>, fog_range: vec4<f32>,
};
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var scene_depth: texture_depth_2d;
@group(1) @binding(1) var<uniform> inverse_view_proj: mat4x4<f32>;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) @interpolate(flat) inverse0: vec4<f32>,
    @location(2) @interpolate(flat) inverse1: vec4<f32>,
    @location(3) @interpolate(flat) inverse2: vec4<f32>,
    @location(4) @interpolate(flat) inverse3: vec4<f32>,
    @location(5) @interpolate(flat) color: vec4<f32>,
    @location(6) @interpolate(flat) handedness: f32,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32,
    @location(0) m0: vec4<f32>, @location(1) m1: vec4<f32>,
    @location(2) m2: vec4<f32>, @location(3) m3: vec4<f32>,
    @location(4) i0: vec4<f32>, @location(5) i1: vec4<f32>,
    @location(6) i2: vec4<f32>, @location(7) i3: vec4<f32>,
    @location(8) color: vec4<f32>, @location(9) handedness: vec4<f32>) -> VsOut {
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
    out.color = color; out.handedness = handedness.x;
    return out;
}

fn haze(in: VsOut, front: bool) -> vec4<f32> {
    if front == (in.handedness > 0.0) { discard; }
    let inverse = mat4x4<f32>(in.inverse0, in.inverse1, in.inverse2, in.inverse3);
    let origin = (inverse * vec4<f32>(globals.camera_pos.xyz, 1.0)).xyz;
    let back = (inverse * vec4<f32>(in.world, 1.0)).xyz;
    let direction = normalize(back - origin);
    let safe = select(vec3<f32>(1e-6), direction, abs(direction) > vec3<f32>(1e-6));
    let a = (vec3<f32>(-0.5) - origin) / safe;
    let b = (vec3<f32>(0.5) - origin) / safe;
    let lo = min(a, b); let hi = max(a, b);
    let start = max(max(max(lo.x, lo.y), lo.z), 0.0);
    var end = min(min(hi.x, hi.y), hi.z);
    let depth = textureLoad(scene_depth, vec2<i32>(in.clip.xy), 0);
    if depth > 0.0 {
        let size = vec2<f32>(textureDimensions(scene_depth));
        let ndc = vec2<f32>(in.clip.x / size.x * 2.0 - 1.0, 1.0 - in.clip.y / size.y * 2.0);
        let hit = inverse_view_proj * vec4<f32>(ndc, depth, 1.0);
        let local_hit = (inverse * vec4<f32>(hit.xyz / hit.w, 1.0)).xyz;
        end = min(end, dot(local_hit - origin, direction));
    }
    if end <= start { return vec4<f32>(0.0); }
    let step = (end - start) / 8.0;
    var sum = 0.0;
    for (var i = 0u; i < 8u; i++) {
        let p = origin + direction * (start + (f32(i) + 0.5) * step);
        let down = clamp(0.5 - p.z, 0.0, 1.0);
        let radius = mix(0.08, 0.48, down);
        let radial = length(p.xy) / radius;
        let soft = 1.0 - smoothstep(0.3, 1.0, radial);
        let vertical = smoothstep(0.0, 0.10, down) * (1.0 - smoothstep(0.68, 1.0, down));
        sum += soft * soft * vertical * step;
    }
    let distance = length(in.world - globals.camera_pos.xyz);
    let fog = clamp((distance - globals.fog_range.x) / max(globals.fog_range.y - globals.fog_range.x, 0.001), 0.0, 1.0);
    return vec4<f32>(in.color.rgb * in.color.a * sum * 0.35 * (1.0 - fog), 0.0);
}

@fragment fn fs_main(in: VsOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> { return haze(in, front); }
@fragment fn fs_main_srgb(in: VsOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let c = haze(in, front);
    let rgb = select(pow((c.rgb + 0.055) / 1.055, vec3<f32>(2.4)), c.rgb / 12.92, c.rgb <= vec3<f32>(0.04045));
    return vec4<f32>(rgb, c.a);
}
