// Analytic world-space effect masks. No game textures or UI shader changes.
struct Globals {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    fog_color: vec4<f32>,
    fog_range: vec4<f32>,
};
@group(0) @binding(0) var<uniform> globals: Globals;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world: vec3<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>, @location(2) uv: vec2<f32>) -> VsOut {
    var out: VsOut;
    out.clip = globals.view_proj * vec4<f32>(position, 1.0);
    out.color = color;
    out.uv = uv;
    out.world = position;
    return out;
}

fn fogged(in: VsOut, alpha: f32) -> vec4<f32> {
    let distance = length(in.world - globals.camera_pos.xyz);
    let fog = clamp((distance - globals.fog_range.x) / max(globals.fog_range.y - globals.fog_range.x, 0.001), 0.0, 1.0);
    return vec4<f32>(mix(in.color.rgb, globals.fog_color.rgb, fog), in.color.a * alpha);
}

@fragment
fn fs_surface(in: VsOut) -> @location(0) vec4<f32> {
    let edge = smoothstep(0.0, 0.12, in.uv.x) * smoothstep(0.0, 0.12, 1.0 - in.uv.x);
    let grooves = 0.8 + 0.2 * sin(in.uv.x * 75.0);
    return fogged(in, edge * grooves);
}

@fragment
fn fs_particle(in: VsOut) -> @location(0) vec4<f32> {
    let p = in.uv * 2.0 - 1.0;
    let radius = dot(p, p);
    let edge = 1.0 - smoothstep(0.05, 1.0, radius);
    let wisps = 0.78 + 0.22 * sin(p.x * 8.0 + sin(p.y * 7.0)) * sin(p.y * 6.0);
    return fogged(in, edge * edge * wisps);
}
