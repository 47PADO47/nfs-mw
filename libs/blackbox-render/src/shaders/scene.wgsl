// Placeholder shading: diffuse texture x vertex colour, one directional light
// plus ambient, linear distance fog. The games' own effects are D3D9 fx_2_0
// programs embedded in their executables (see docs/formats/shaders.md).

// Set per pipeline: pre-lit world geometry (vertex colour x 2, no sun) vs lit models.
override PRELIT: bool = false;

struct Globals {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    fog_color: vec4<f32>,
    // x = start, y = end
    fog_range: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var diffuse: texture_2d<f32>;
@group(1) @binding(1) var diffuse_sampler: sampler;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color_bgra: vec4<f32>,
    @location(3) uv: vec2<f32>,
    // Per-instance model matrix, one column per attribute.
    @location(4) model_0: vec4<f32>,
    @location(5) model_1: vec4<f32>,
    @location(6) model_2: vec4<f32>,
    @location(7) model_3: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world: vec3<f32>,
};

@vertex
fn vs_main(v: VsIn) -> VsOut {
    let model = mat4x4<f32>(v.model_0, v.model_1, v.model_2, v.model_3);
    let world = model * vec4<f32>(v.position, 1.0);
    var out: VsOut;
    out.clip = globals.view_proj * world;
    out.normal = (model * vec4<f32>(v.normal, 0.0)).xyz;
    out.color = v.color_bgra.zyxw;
    out.uv = v.uv;
    out.world = world.xyz;
    return out;
}

fn shade(in: VsOut) -> vec4<f32> {
    let base = textureSample(diffuse, diffuse_sampler, in.uv) * in.color;
    var lit: vec3<f32>;
    if PRELIT {
        lit = min(base.rgb * 2.0, vec3<f32>(1.0));
    } else {
        let n = normalize(in.normal);
        let diffuse_light = max(dot(n, -globals.light_dir.xyz), 0.0);
        // Thin parts are often seen from behind: give back faces a little light.
        let back_light = max(dot(-n, -globals.light_dir.xyz), 0.0) * 0.25;
        lit = base.rgb * (0.35 + 0.65 * diffuse_light + back_light);
    }
    let distance = length(in.world - globals.camera_pos.xyz);
    let fog = clamp((distance - globals.fog_range.x) / max(globals.fog_range.y - globals.fog_range.x, 0.001), 0.0, 1.0);
    return vec4<f32>(mix(lit, globals.fog_color.rgb, fog), base.a);
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
