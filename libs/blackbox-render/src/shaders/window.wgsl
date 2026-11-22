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
@group(1) @binding(3) var window_environment: texture_cube<f32>;
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
// Recover the origin of the authored UV plane. A continuous shop display gets one
// switch, while separately mapped windows can have different occupants.
fn room_seed(world: vec3<f32>, uv: vec2<f32>) -> f32 {
    let x = dpdx(uv);
    let y = dpdy(uv);
    let det = x.x*y.y - x.y*y.x;
    let du = dpdx(world)*y.y - dpdy(world)*x.y;
    let dv = dpdy(world)*x.x - dpdx(world)*y.x;
    let origin = world - (du*uv.x + dv*uv.y) / select(1.0, det, abs(det)>1e-14);
    let cell = floor(origin * 0.5 + 0.5);
    let p = fract(cell * vec3(0.1031, 0.1030, 0.0973));
    let h = p + dot(p, p.yzx + vec3(33.33));
    return select(0.0, fract((h.x+h.y)*h.z), abs(det)>1e-14);
}

fn shade(v: Output) -> vec4<f32> {
    if CLIP_PLANE && dot(globals.clip_plane, vec4<f32>(v.world, 1.0)) < 0.0 { discard; }
    let tex = textureSampleBias(diffuse_texture,material_sampler,v.uv,globals.fog_range.z);

    let plane = receiver_normal(v.world,v.normal);
    if globals.fog_range.w<0.5 && shadow_data.debug.x>0.5 {
        return vec4<f32>(lighting_debug_colour(v.world,plane,v.clip.xy),1.0);
    }
    var q = 1.0;
    if globals.fog_range.w<0.5 && v.receives_shadow != 0u { q = direct_visibility(v.world,plane,v.clip.xy); }
    q *= cloud_visibility(v.world);
    // Daytime sky reflections and in-scattering are baked into these authored windows.
    // Reduce those two terms at night while retaining the source window pattern.
    let night = shadow_data.local_options.w;
    var reflected: vec3<f32>;
    var base_weight = 1.0;
    var highlight = v.highlight;
    if globals.camera_right.w > 0.5 {
        let eye = unit(globals.camera_pos.xyz - v.world);
        let authored_normal = unit(v.normal);
        let normal = select(plane, authored_normal, abs(dot(plane, authored_normal)) > 0.98);
        let ray = reflect(-eye, normal);
        let edge = 1.0 - clamp(abs(dot(eye, normal)), 0.0, 1.0);
        let fresnel = 0.12 + 0.32 * edge * edge * edge;
        let weight = tex.a * fresnel;
        let horizon = globals.fog_color.rgb;
        let sky = mix(horizon, horizon * vec3(0.66, 0.83, 1.12), smoothstep(0.0, 0.8, ray.z));
        let ground = mix(horizon * 0.14, globals.surface_ambient.rgb * 0.35, 0.5);
        var outside = mix(ground, sky, smoothstep(-0.18, 0.12, ray.z));
        if globals.camera_right.w > 1.5 {
            let levels = textureNumLevels(window_environment);
            if levels > 1u {
                // Keep only large-scale surroundings. This optional pyramid already exists for car paint.
                let lod = min(f32(levels-1u), log2(f32(textureDimensions(window_environment).x)) - 3.5);
                let broad = textureSampleLevel(window_environment, material_sampler, ray, max(lod,0.0));
                outside = mix(outside, broad.rgb, 0.55*broad.a);
            } else {
                let axes = ray * ray;
                let dir = select(vec3(-1.0), vec3(1.0), ray >= vec3(0.0));
                let distant = axes.x * textureSampleLevel(window_environment, material_sampler, vec3(dir.x,0,0),0).rgb
                    + axes.y * textureSampleLevel(window_environment, material_sampler, vec3(0,dir.y,0),0).rgb
                    + axes.z * textureSampleLevel(window_environment, material_sampler, vec3(0,0,dir.z),0).rgb;
                outside = mix(outside, distant, 0.65);
            }
        }
        // Trace to a backing plane 12 cm behind the glass. Only the opening mask moves:
        // frames, lettering and merchandise can share a single diffuse atlas.
        let dx = dpdx(v.world);
        let dy = dpdy(v.world);
        let area = cross(dx, dy);
        let depth = 0.12 / max(abs(dot(eye, normal)), 0.28);
        let travel = depth * (dot(eye, normal)*normal - eye);
        let inv_area = area / max(dot(area, area), 1e-16);
        let uv_shift = dpdx(v.uv)*dot(cross(travel, dy),inv_area) + dpdy(v.uv)*dot(cross(dx,travel),inv_area);
        let behind = textureSampleBias(diffuse_texture,material_sampler,v.uv + uv_shift,globals.fog_range.z).a;
        let reveal = 1.0 - 0.38 * max(tex.a - behind, 0.0);
        base_weight = (1.0 - weight) * reveal;
        let surface = 2.0 * tex.rgb * (v.ambient + v.diffuse*q);
        // The photographed display behind the pane has its own diffuse light, even under an awning.
        let display = max(surface, 0.60 * sqrt(tex.rgb));
        let interior = mix(surface, display, smoothstep(0.02,0.55,tex.a) * (1.0-night)) * reveal;
        let glass = sqrt((1.0-weight) * interior * interior + weight * outside * outside);
        reflected = glass - base_weight*surface;
        let sun = max(dot(ray, unit(-globals.light_dir.xyz)), 0.0);
        highlight = fresnel * pow(sun, 96.0);
    } else {
        let reflection = textureSampleBias(reflection_texture,material_sampler,v.reflection_uv,globals.fog_range.z);
        reflected = tex.a * reflection.rgb * mix(1.0, 0.45, night);
    }
    let scatter = v.scatter * mix(1.0, 0.035, night);
    var rgb = 2.0*(base_weight*tex.rgb*(v.ambient+v.diffuse*q)+2.0*tex.a*highlight*q)+reflected+scatter;
    if shadow_data.local_options.w > 0.0 {
        rgb *= mix(vec3(1.0), vec3(0.30, 0.35, 0.45), shadow_data.local_options.w);
        if globals.fog_range.w < 0.5 { rgb += shadow_data.local_options.w*2.0*tex.rgb*local_lighting(v.world,plane); }
    }
    if globals.camera_right.w > 0.5 && night > 0.0 {
        let seed = room_seed(v.world, v.uv);
        let fade = smoothstep(0.08 + 0.32*seed, 0.66 + 0.28*seed, night);
        let luminance = dot(tex.rgb, vec3(0.299, 0.587, 0.114));
        let lamp = mix(vec3(1.0, 0.56, 0.27), vec3(1.0, 0.83, 0.59), seed);
        let glow = (0.30*sqrt(luminance) + 1.5*luminance) * tex.a;
        rgb += select(vec3(0.0), fade * glow * lamp * base_weight, seed > 0.42);
    }
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
