// Authored world glazing; docs/specs/world-window-materials.md.
// Set only for planar-reflection captures: discard fragments behind `globals.clip_plane`. Left off elsewhere so
// the opaque draws of the main view keep early depth testing.
override CLIP_PLANE: bool = false;
struct Globals {
    view_proj: mat4x4<f32>, camera_pos: vec4<f32>, light_dir: vec4<f32>,
    fog_color: vec4<f32>, fog_range: vec4<f32>, scatter_rayleigh: vec4<f32>,
    scatter_mie: vec4<f32>, to_sun: vec4<f32>, surface_ambient: vec4<f32>,
    surface_diffuse: vec4<f32>, camera_right: vec4<f32>, clip_plane: vec4<f32>,
};
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var diffuse_texture: texture_2d<f32>;
@group(1) @binding(1) var reflection_texture: texture_2d<f32>;
@group(1) @binding(2) var material_sampler: sampler;
struct Input {
    @location(0) position: vec3<f32>, @location(1) normal: vec3<f32>,
    @location(2) color_bgra: vec4<f32>, @location(3) uv: vec2<f32>,
};
// instance input: begin
// The per-instance model matrix, one column per attribute. A renderer with an instance store replaces this block
// (see `instance_view.rs`): the instance then carries a slot at location 4 and the matrix is read from the store.
struct InstanceIn {
    @location(4) model_0: vec4<f32>, @location(5) model_1: vec4<f32>,
    @location(6) model_2: vec4<f32>, @location(7) model_3: vec4<f32>,
    @location(12) shadow_flags: vec4<f32>,
};
fn instance_model(i: InstanceIn) -> mat4x4<f32> { return mat4x4<f32>(i.model_0, i.model_1, i.model_2, i.model_3); }
fn instance_receives_shadow(i: InstanceIn) -> u32 { return u32(i.shadow_flags.x); }
// instance input: end
struct Output {
    @invariant @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>, @location(1) reflection_uv: vec2<f32>,
    @location(2) ambient: vec3<f32>, @location(3) diffuse: vec3<f32>,
    @location(4) highlight: f32, @location(5) scatter: vec3<f32>,
    @location(6) world: vec3<f32>, @location(7) normal: vec3<f32>,
    @location(8) @interpolate(flat) receives_shadow: u32,
};
struct Atmosphere { extinction: vec3<f32>, scatter: vec3<f32> };
fn unit(v: vec3<f32>) -> vec3<f32> { return v / max(length(v), 1e-9); }
fn atmosphere(delta: vec3<f32>) -> Atmosphere {
    var out: Atmosphere;
    out.extinction = vec3(1.0);
    out.scatter = vec3(0.0);
    if globals.scatter_rayleigh.w < 0.5 { return out; }
    let d = clamp(length(delta)*globals.to_sun.w,0.0,2500.0);
    let c = dot(unit(delta),globals.to_sun.xyz);
    let br = globals.scatter_rayleigh.rgb;
    let bm = globals.scatter_mie.rgb;
    let g = globals.scatter_mie.w;
    let sum = br+bm;
    out.extinction = min(exp(-sum*d),vec3(1.0));
    let rayleigh = 3.0/(16.0*3.14159265358979)*(1.0+c*c);
    let mie = (1.0-g)*(1.0-g)/(4.0*3.14159265358979*pow(max(1.0+g*g-2.0*g*c,1e-6),1.5));
    out.scatter = clamp((br*rayleigh+bm*mie)*(vec3(1.0)-out.extinction)
        /max(sum,vec3(1e-9))*globals.surface_diffuse.w,vec3(0.0),vec3(1.0));
    return out;
}
@vertex
fn vs_main(v: Input, placed: InstanceIn) -> Output {
    let model = instance_model(placed);
    let world = model*vec4(v.position,1.0);
    let normal = (model*vec4(v.normal,0.0)).xyz;
    let delta = globals.camera_pos.xyz-world.xyz;
    let light = -globals.light_dir.xyz;
    let dot_light = dot(normal,light);
    let reflected_light = 2.0*dot_light*normal-light;
    let h = clamp(dot(reflected_light,unit(delta)),0.0,1.0);
    let fog = atmosphere(delta);
    let ambient = select(vec3(0.35),globals.surface_ambient.rgb*globals.fog_color.w,globals.surface_ambient.w>0.5);
    let diffuse = select(vec3(0.65),globals.surface_diffuse.rgb,globals.surface_ambient.w>0.5);
    var out: Output;
    out.receives_shadow = instance_receives_shadow(placed);
    out.clip = globals.view_proj*world;
    out.world = world.xyz;
    out.normal = normal;
    out.uv = v.uv;
    out.reflection_uv = 0.5*v.uv+vec2(0.5*dot(normal,globals.camera_right.xyz),0.0);
    out.ambient = clamp(ambient*v.color_bgra.zyx*fog.extinction,vec3(0.0),vec3(1.0));
    out.diffuse = clamp(diffuse*clamp(dot_light,0.0,1.0),vec3(0.0),vec3(1.0));
    out.highlight = pow(h,10.0);
    out.scatter = fog.scatter;
    return out;
}
fn shade(v: Output) -> vec4<f32> {
    if CLIP_PLANE && dot(globals.clip_plane, vec4<f32>(v.world, 1.0)) < 0.0 { discard; }
    let tex = textureSampleBias(diffuse_texture,material_sampler,v.uv,globals.fog_range.z);
    let reflection = textureSampleBias(reflection_texture,material_sampler,v.reflection_uv,globals.fog_range.z);
    let plane = receiver_normal(v.world,v.normal);
    if globals.fog_range.w<0.5 && shadow_data.debug.x>0.5 {
        return vec4<f32>(lighting_debug_colour(v.world,plane,v.clip.xy),1.0);
    }
    var q = 1.0;
    if globals.fog_range.w<0.5 && v.receives_shadow != 0u { q = direct_visibility(v.world,plane,v.clip.xy); }
    q *= cloud_visibility(v.world);
    let rgb = 2.0*(tex.rgb*v.ambient+(tex.rgb*v.diffuse+2.0*tex.a*v.highlight)*q)+tex.a*reflection.rgb+v.scatter;
    return extended_output(vec4(rgb,tex.a));
}
@fragment fn fs_opaque(v: Output) -> @location(0) vec4<f32> { return shade(v); }
@fragment fn fs_alpha_test(v: Output) -> @location(0) vec4<f32> {
    let color = shade(v);
    if color.a < 0.5 { discard; }
    return color;
}
@fragment fn fs_blend(v: Output) -> @location(0) vec4<f32> { return shade(v); }

// HDR-only extension of final RGB; intermediate authored arithmetic remains intact.
fn extended_output(c:vec4<f32>)->vec4<f32> {
    if globals.camera_pos.w>1.5 { return vec4(max(c.rgb,vec3(0.0)),clamp(c.a,0.0,1.0)); }
    return clamp(c,vec4(0.0),vec4(1.0));
}
