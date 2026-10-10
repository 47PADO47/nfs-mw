// Analytic world-space effect masks. No game textures or UI shader changes.
struct Globals {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    fog_color: vec4<f32>,
    fog_range: vec4<f32>,
};
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var particle_texture: texture_2d<f32>;
@group(1) @binding(1) var particle_sampler: sampler;

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

// The exact sRGB decode, so decode then the hardware's encode on write is the identity (a renderer whose
// render target view is sRGB compiles this module with `SRGB_TARGET` set; see `world::is_srgb`).
fn linear_from_gamma(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

fn surface_color(in: VsOut) -> vec4<f32> {
    let edge = smoothstep(0.0, 0.12, in.uv.x) * smoothstep(0.0, 0.12, 1.0 - in.uv.x);
    let grooves = 0.8 + 0.2 * sin(in.uv.x * 75.0);
    return fogged(in, edge * grooves);
}

@fragment
fn fs_surface(in: VsOut) -> @location(0) vec4<f32> {
    let c = surface_color(in);
    @if(SRGB_TARGET) {
        return vec4<f32>(linear_from_gamma(c.rgb), c.a);
    } @else {
        return c;
    }
}

fn particle_color(in: VsOut) -> vec4<f32> {
    let p = in.uv * 2.0 - 1.0;
    let radius = dot(p, p);
    let edge = 1.0 - smoothstep(0.05, 1.0, radius);
    let wisps = 0.78 + 0.22 * sin(p.x * 8.0 + sin(p.y * 7.0)) * sin(p.y * 6.0);
    return fogged(in, edge * edge * wisps);
}

@fragment
fn fs_particle(in: VsOut) -> @location(0) vec4<f32> {
    let c = particle_color(in);
    @if(SRGB_TARGET) {
        return vec4<f32>(linear_from_gamma(c.rgb), c.a);
    } @else {
        return c;
    }
}

fn streak_color(in: VsOut) -> vec4<f32> {
    let along = clamp(in.uv.y, 0.0, 1.0);
    let across = in.uv.x * 2.0 - 1.0;
    // Narrow luminous core with a dim continuous tail; mean energy stays near one tenth.
    let mask = exp(-8.0 * across * across - 3.0 * along) * (1.0 - smoothstep(0.85, 1.0, along));
    let distance = length(in.world - globals.camera_pos.xyz);
    let fog = clamp((distance - globals.fog_range.x) / max(globals.fog_range.y - globals.fog_range.x, 0.001), 0.0, 1.0);
    // Additive light disappears into fog; blending toward the fog colour would emit luminous fog.
    return vec4<f32>(in.color.rgb * (1.0 - fog), in.color.a * mask);
}

@fragment
fn fs_streak(in: VsOut) -> @location(0) vec4<f32> {
    let c = streak_color(in);
    @if(SRGB_TARGET) {
        return vec4<f32>(linear_from_gamma(c.rgb), c.a);
    } @else {
        return c;
    }
}

fn glow_color(in: VsOut) -> vec4<f32> {
    let p = in.uv * 2.0 - 1.0;
    let radius = dot(p, p);
    let edge = 1.0 - smoothstep(0.0, 1.0, radius);
    let core = exp(-radius * 12.0);
    let mask = edge * edge * (0.25 + 0.75 * core);
    let distance = length(in.world - globals.camera_pos.xyz);
    let fog = clamp((distance - globals.fog_range.x) / max(globals.fog_range.y - globals.fog_range.x, 0.001), 0.0, 1.0);
    return vec4<f32>(in.color.rgb * (1.0 - fog), in.color.a * mask);
}

@fragment
fn fs_glow(in: VsOut) -> @location(0) vec4<f32> {
    let c = glow_color(in);
    @if(SRGB_TARGET) {
        return vec4<f32>(linear_from_gamma(c.rgb), c.a);
    } @else {
        return c;
    }
}

fn textured_color(in: VsOut) -> vec4<f32> {
    let texel = textureSample(particle_texture, particle_sampler, in.uv);
    let distance = length(in.world - globals.camera_pos.xyz);
    let fog = clamp((distance - globals.fog_range.x) / max(globals.fog_range.y - globals.fog_range.x, 0.001), 0.0, 1.0);
    return vec4<f32>(texel.rgb * in.color.rgb * (1.0 - fog), texel.a * in.color.a);
}

@fragment
fn fs_textured(in: VsOut) -> @location(0) vec4<f32> {
    let c = textured_color(in);
    @if(SRGB_TARGET) {
        return vec4<f32>(linear_from_gamma(c.rgb), c.a);
    } @else {
        return c;
    }
}

fn textured_alpha_color(in: VsOut) -> vec4<f32> {
    let texel = textureSample(particle_texture, particle_sampler, in.uv);
    let distance = length(in.world - globals.camera_pos.xyz);
    let fog = clamp((distance - globals.fog_range.x) / max(globals.fog_range.y - globals.fog_range.x, 0.001), 0.0, 1.0);
    return vec4<f32>(mix(texel.rgb * in.color.rgb, globals.fog_color.rgb, fog), texel.a * in.color.a);
}

@fragment
fn fs_textured_alpha(in: VsOut) -> @location(0) vec4<f32> {
    let c = textured_alpha_color(in);
    @if(SRGB_TARGET) {
        return vec4<f32>(linear_from_gamma(c.rgb), c.a);
    } @else {
        return c;
    }
}
