// Placeholder shading: diffuse texture x vertex colour, one directional light
// plus ambient, linear distance fog. The games' own effects are D3D9 fx_2_0
// programs embedded in their executables (see docs/formats/shaders.md).

// Set per pipeline: pre-lit world geometry (vertex colour x 2, no sun) vs lit models.
override PRELIT: bool = false;
// Off for the sky, which sits beyond the fog.
override FOG: bool = true;

struct Globals {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    fog_color: vec4<f32>,
    // x = start, y = end, z = texture LOD bias
    fog_range: vec4<f32>,
    // Scattering fog (see the Rust `Globals`): Rayleigh xyz with w = on, Mie xyz with w = g, sun direction.
    scatter_rayleigh: vec4<f32>,
    scatter_mie: vec4<f32>,
    to_sun: vec4<f32>,
    // Lighting colours of the world surfaces: ambient xyz with w = 1 when on, diffuse xyz.
    surface_ambient: vec4<f32>,
    surface_diffuse: vec4<f32>,
};

const PI_F: f32 = 3.14159265358979;

/// The atmospheric scattering fog (`docs/specs/time-of-day.md`): `colour·E + in-scattered`, with
/// `E = exp(−(Br + Bm)·d)` and the in-scattered light for the angle `c` between the view ray and the sun.
fn scatter_fog(colour: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    let offset = world - globals.camera_pos.xyz;
    let d = length(offset);
    let view = offset / max(d, 1e-4);
    let c = dot(view, globals.to_sun.xyz);
    let br = globals.scatter_rayleigh.xyz;
    let bm = globals.scatter_mie.xyz;
    let g = globals.scatter_mie.w;
    let sum = max(br + bm, vec3<f32>(1e-9));
    let e = exp(-(br + bm) * d);
    let hg = (1.0 - g) * (1.0 - g) / (4.0 * PI_F * pow(max(1.0 + g * g - 2.0 * g * c, 1e-6), 1.5));
    let rayleigh = 3.0 / (16.0 * PI_F) * (1.0 + c * c);
    let inscatter = (br * rayleigh + bm * hg) * (vec3<f32>(1.0) - e) / sum;
    return colour * e + inscatter;
}

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
    let base = textureSampleBias(diffuse, diffuse_sampler, in.uv, globals.fog_range.z) * in.color;
    var lit: vec3<f32>;
    if PRELIT {
        lit = min(base.rgb * 2.0, vec3<f32>(1.0));
    } else {
        let n = normalize(in.normal);
        let diffuse_light = max(dot(n, -globals.light_dir.xyz), 0.0);
        // Thin parts are often seen from behind: give back faces a little light.
        let back_light = max(dot(-n, -globals.light_dir.xyz), 0.0) * 0.25;
        // The game's ambient and diffuse colours when the lighting is on; the fixed 0.35 and 0.65 otherwise.
        let on = globals.surface_ambient.w > 0.5;
        let ambient = select(vec3<f32>(0.35), globals.surface_ambient.xyz, on);
        let diffuse = select(vec3<f32>(0.65), globals.surface_diffuse.xyz, on);
        lit = base.rgb * (ambient + diffuse * diffuse_light + back_light);
    }
    let distance = length(in.world - globals.camera_pos.xyz);
    var fog = clamp((distance - globals.fog_range.x) / max(globals.fog_range.y - globals.fog_range.x, 0.001), 0.0, 1.0);
    if !FOG {
        fog = 0.0;
    }
    if FOG && globals.scatter_rayleigh.w > 0.5 {
        return vec4<f32>(scatter_fog(lit, in.world), base.a);
    }
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
