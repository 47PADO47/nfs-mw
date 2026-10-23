// Authored atmospheric sky behavior; docs/specs/world-sky-shading.md.
struct Globals {
    view_proj: mat4x4<f32>, camera_pos: vec4<f32>, light_dir: vec4<f32>,
    fog_color: vec4<f32>, fog_range: vec4<f32>, scatter_rayleigh: vec4<f32>,
    scatter_mie: vec4<f32>, to_sun: vec4<f32>, surface_ambient: vec4<f32>, surface_diffuse: vec4<f32>,
};
struct Material { intensity: vec4<f32>, options: vec4<f32> };
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var diffuse: texture_2d<f32>;
@group(1) @binding(1) var miscellaneous: texture_2d<f32>;
@group(1) @binding(2) var sky_sampler: sampler;
@group(1) @binding(3) var<uniform> material: Material;

struct Input {
    @location(0) position: vec3<f32>, @location(3) uv: vec2<f32>,
    @location(8) secondary_uv: vec2<f32>,
};
// instance input: begin
// The per-instance model matrix, one column per attribute. A renderer with an instance store replaces this block
// (see `instance_view.rs`): the instance then carries a slot at location 4 and the matrix is read from the store.
struct InstanceIn {
    @location(4) model_0: vec4<f32>, @location(5) model_1: vec4<f32>,
    @location(6) model_2: vec4<f32>, @location(7) model_3: vec4<f32>,
};
fn instance_model(i: InstanceIn) -> mat4x4<f32> { return mat4x4<f32>(i.model_0, i.model_1, i.model_2, i.model_3); }
// instance input: end
struct Output {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv0: vec2<f32>, @location(1) uv1: vec2<f32>,
    @location(2) uv2: vec2<f32>, @location(3) background: vec3<f32>,
};

fn atmosphere(world: vec3<f32>) -> vec3<f32> {
    if globals.scatter_rayleigh.w < 0.5 { return globals.fog_color.rgb; }
    let to_eye = globals.camera_pos.xyz - world;
    let light = globals.to_sun.xyz / max(length(globals.to_sun.xyz), 1e-6);
    let c = dot(to_eye / max(length(to_eye), 1e-6), light);
    let br = globals.scatter_rayleigh.rgb;
    let bm = globals.scatter_mie.rgb;
    let g = globals.scatter_mie.w;
    let rayleigh = 3.0 / (16.0 * 3.14159265358979) * (1.0 + c*c);
    let mie = (1.0-g)*(1.0-g) / (4.0 * 3.14159265358979 * pow(max(1.0+g*g-2.0*g*c, 1e-6), 1.5));
    let extinction = exp(-15000.0 * (br+bm));
    return clamp((br*rayleigh+bm*mie) * (vec3(1.0)-extinction) / max(br+bm, vec3(1e-9)) * globals.surface_diffuse.w, vec3(0.0), vec3(1.0));
}

@vertex
fn vs_main(v: Input, placed: InstanceIn) -> Output {
    let model = instance_model(placed);
    let world = model * vec4(v.position, 1.0);
    let scroll = material.options.y;
    var out: Output;
    out.clip = globals.view_proj * world;
    out.uv0 = v.uv - vec2(scroll, 0.0);
    out.uv1 = v.secondary_uv;
    out.uv2 = v.uv;
    out.background = atmosphere(world.xyz);
    if material.options.x < 0.5 {
        out.uv0 = v.uv - vec2(0.4*scroll, 0.0);
        out.uv2 = v.uv - vec2(0.2*scroll+0.3, 0.0);
    }
    return out;
}

@fragment
fn fs_main(in: Output) -> @location(0) vec4<f32> {
    if material.options.x > 1.5 {
        let tex = textureSample(miscellaneous, sky_sampler, in.uv1);
        return extended_output(vec4(tex.rgb, tex.a * material.intensity.z));
    }
    let tex = textureSample(diffuse, sky_sampler, in.uv0);
    if material.options.x > 0.5 {
        return extended_output(vec4(tex.rgb, tex.a * material.intensity.y));
    }
    let second = textureSample(diffuse, sky_sampler, in.uv2);
    let misc = textureSample(miscellaneous, sky_sampler, in.uv1);
    let a = 2.0 * tex.a * second.a * material.intensity.x;
    let cloud = (misc.rgb + tex.rgb) * 0.5;
    return extended_output(vec4(cloud*a + in.background*(1.0-a), 1.0));
}

// HDR-only extension of final RGB; intermediate authored arithmetic remains intact.
fn extended_output(c:vec4<f32>)->vec4<f32> {
    if globals.camera_pos.w>1.5 { return vec4(max(c.rgb,vec3(0.0)),clamp(c.a,0.0,1.0)); }
    return clamp(c,vec4(0.0),vec4(1.0));
}
