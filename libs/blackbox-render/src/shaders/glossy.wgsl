// Glossy shading: three directional lights, a sun highlight and an environment reflection,
// every ramp driven by how much the surface faces the viewer. Written from
// docs/specs/car-assembly.md section 8. Output stays in the colour range of the target: no
// tone mapping here.

struct Globals {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    fog_color: vec4<f32>,
    // x = start, y = end
    fog_range: vec4<f32>,
    // Scattering fog (see the Rust `Globals`): Rayleigh xyz with w = on, Mie xyz with w = g, sun direction.
    scatter_rayleigh: vec4<f32>,
    scatter_mie: vec4<f32>,
    to_sun: vec4<f32>,
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

// Three lights: direction towards the light (xyz), colour (xyz), then the ambient colour.
struct Rig {
    to_light: array<vec4<f32>, 3>,
    color: array<vec4<f32>, 3>,
    ambient: vec4<f32>,
};

struct Material {
    diffuse_min: vec4<f32>,
    diffuse_range: vec4<f32>,
    specular_min: vec4<f32>,
    specular_range: vec4<f32>,
    // x = specular power, y = envmap min, z = envmap range, w = envmap power
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var diffuse: texture_2d<f32>;
@group(1) @binding(1) var diffuse_sampler: sampler;
@group(2) @binding(0) var<uniform> rig: Rig;
@group(2) @binding(1) var env_map: texture_cube<f32>;
@group(2) @binding(2) var env_sampler: sampler;
struct Irradiance { coefficients: array<vec4<f32>, 18>, };
@group(2) @binding(3) var<uniform> irradiance: Irradiance;

fn environment_light(n: vec3<f32>) -> vec3<f32> {
    let basis = array<f32, 9>(0.282095, 0.488603 * n.y, 0.488603 * n.z, 0.488603 * n.x,
        1.092548 * n.x * n.y, 1.092548 * n.y * n.z, 0.315392 * (3.0 * n.z * n.z - 1.0),
        1.092548 * n.x * n.z, 0.546274 * (n.x * n.x - n.y * n.y));
    var light = vec3<f32>(0.0);
    for (var i = 0u; i < 9u; i++) {
        light += basis[i] * (irradiance.coefficients[i].xyz * shadow_data.environment.y
            + irradiance.coefficients[i + 9u].xyz * shadow_data.environment.z);
    }
    return max(light, vec3<f32>(0.0));
}
@group(3) @binding(0) var<uniform> material: Material;

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
    @invariant @builtin(position) clip: vec4<f32>,
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

struct Shaded {
    rgb: vec3<f32>,
    // The texture's own alpha, for the alpha test.
    texture_alpha: f32,
    // Texture alpha times the material's diffuse alpha, for blending.
    alpha: f32,
};

fn shade(in: VsOut, receive_ao: bool) -> Shaded {
    let tex = textureSample(diffuse, diffuse_sampler, in.uv);
    let n = normalize(in.normal);
    let v = normalize(globals.camera_pos.xyz - in.world);

    // How much the surface faces the viewer.
    let f = max(dot(n, v), 0.01);
    // Baked occlusion: vertex colours of one half and above leave the surface unshadowed.
    let occlusion = saturate(2.0 * in.color.r);

    let receiver = receiver_normal(in.world, n);
    let visibility = direct_visibility(in.world, receiver, in.clip.xy);
    var ao = 1.0;
    if receive_ao { ao = ambient_visibility(in.clip.xy); }
    var indirect = rig.ambient.xyz;
    for (var k = 1; k < 3; k++) {
        indirect += saturate(dot(n, rig.to_light[k].xyz) + 0.1) * rig.color[k].xyz;
    }
    if shadow_data.environment.x > 0.5 { indirect = rig.ambient.xyz + environment_light(n); }
    let direct = saturate(dot(n, rig.to_light[0].xyz) + 0.1) * rig.color[0].xyz * visibility;
    let light = direct + indirect * ao;

    let diff = material.diffuse_min + material.diffuse_range * f;
    let spec = material.specular_min.xyz + material.specular_range.xyz * pow(f, material.params.x);
    let env_strength = material.params.y + material.params.z * pow(f, material.params.w);

    // Sun highlight: the sun mirrored about the normal, against the view direction. Only for
    // surfaces the sun reaches.
    let sun = rig.to_light[0].xyz;
    let n_sun = dot(n, sun);
    let mirrored = 2.0 * n_sun * n - sun;
    let glint = visibility * 2.0 * pow(saturate(dot(mirrored, v)), 16.0) * step(0.0, n_sun);

    // Environment reflection of the view vector mirrored about the normal.
    let reflected = 2.0 * f * n - v;
    let env = textureSample(env_map, env_sampler, reflected).rgb;

    var out: Shaded;
    out.rgb = occlusion * light * diff.rgb * tex.rgb
        + glint * diff.a * occlusion * spec
        + env * (occlusion * env_strength * diff.a * 0.5);
    if shadow_data.options.w > 0.5 {
        let fresnel = 0.04 + 0.96 * pow(1.0 - saturate(dot(n, v)), 5.0);
        let h = normalize(v + rig.to_light[0].xyz);
        let ndh = saturate(dot(n, h));
        let a2 = 0.0144;
        let d = a2 / (3.14159265 * pow(ndh * ndh * (a2 - 1.0) + 1.0, 2.0));
        let coat = min(d * 0.04 * saturate(n_sun) * 0.25, 2.0) * visibility;
        out.rgb = out.rgb * (1.0 - fresnel * 0.4) + env * fresnel * 0.4 + rig.color[0].xyz * coat;
    }
    out.texture_alpha = tex.a;
    out.alpha = tex.a * diff.a;

    if shadow_data.debug.x > 0.5 {
        out.rgb = lighting_debug_colour(in.world, receiver, in.clip.xy);
        return out;
    }

    let distance = length(in.world - globals.camera_pos.xyz);
    let fog = clamp((distance - globals.fog_range.x) / max(globals.fog_range.y - globals.fog_range.x, 0.001), 0.0, 1.0);
    if globals.scatter_rayleigh.w > 0.5 {
        out.rgb = scatter_fog(out.rgb, in.world);
        return out;
    }
    out.rgb = mix(out.rgb, globals.fog_color.rgb, fog);
    return out;
}

@fragment
fn fs_opaque(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(shade(in, true).rgb, 1.0);
}

@fragment
fn fs_alpha_test(in: VsOut) -> @location(0) vec4<f32> {
    let c = shade(in, true);
    if c.texture_alpha < 0.5 {
        discard;
    }
    return vec4<f32>(c.rgb, 1.0);
}

@fragment
fn fs_blend(in: VsOut) -> @location(0) vec4<f32> {
    let c = shade(in, false);
    return vec4<f32>(c.rgb, c.alpha);
}
