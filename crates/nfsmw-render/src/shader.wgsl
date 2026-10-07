// Placeholder shading for milestone 1: diffuse texture x vertex colour, one
// directional light plus ambient. The game's own effects (CarShader, WorldShader,
// ...) are D3D9 fx_2_0 programs embedded in speed.exe; see docs/formats/shaders.md.

struct Globals {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var diffuse: texture_2d<f32>;
@group(1) @binding(1) var diffuse_sampler: sampler;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color_bgra: vec4<f32>,
    @location(3) uv: vec2<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var out: VsOut;
    out.clip = globals.view_proj * vec4<f32>(v.position, 1.0);
    out.normal = v.normal;
    out.color = v.color_bgra.zyxw;
    out.uv = v.uv;
    return out;
}

fn shade(in: VsOut) -> vec4<f32> {
    let base = textureSample(diffuse, diffuse_sampler, in.uv) * in.color;
    let n = normalize(in.normal);
    let diffuse_light = max(dot(n, -globals.light_dir.xyz), 0.0);
    // Two-sided-ish: car interiors and thin parts are often seen from behind.
    let back_light = max(dot(-n, -globals.light_dir.xyz), 0.0) * 0.25;
    let light = 0.35 + 0.65 * diffuse_light + back_light;
    return vec4<f32>(base.rgb * light, base.a);
}

@fragment
fn fs_opaque(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(shade(in).rgb, 1.0);
}

@fragment
fn fs_alpha_test(in: VsOut) -> @location(0) vec4<f32> {
    let c = shade(in);
    if c.a < 0.5 {
        discard;
    }
    return vec4<f32>(c.rgb, 1.0);
}

@fragment
fn fs_blend(in: VsOut) -> @location(0) vec4<f32> {
    return shade(in);
}
