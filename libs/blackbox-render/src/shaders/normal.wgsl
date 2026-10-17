// Tangent-space scenery; docs/specs/world-normal-materials.md.
struct Globals {
    view_proj: mat4x4<f32>, camera_pos: vec4<f32>, light_dir: vec4<f32>,
    fog_color: vec4<f32>, fog_range: vec4<f32>, scatter_rayleigh: vec4<f32>,
    scatter_mie: vec4<f32>, to_sun: vec4<f32>, surface_ambient: vec4<f32>, surface_diffuse: vec4<f32>,
};
struct Offset { value: vec4<f32> };
struct Light { specular: vec4<f32>, reflection: vec4<f32> };
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var diffuse_texture: texture_2d<f32>;
@group(1) @binding(1) var normal_texture: texture_2d<f32>;
@group(1) @binding(2) var specular_texture: texture_2d<f32>;
@group(1) @binding(3) var material_sampler: sampler;
@group(1) @binding(4) var<uniform> uv_offset: Offset;
@group(1) @binding(5) var<uniform> light: Light;
@group(1) @binding(6) var reflection_texture: texture_2d<f32>;
struct Input {
    @location(0) position: vec3<f32>, @location(1) normal: vec3<f32>,
    @location(2) color_bgra: vec4<f32>, @location(3) uv: vec2<f32>,
    @location(4) model_0: vec4<f32>, @location(5) model_1: vec4<f32>,
    @location(6) model_2: vec4<f32>, @location(7) model_3: vec4<f32>,
    @location(8) tangent: vec4<f32>,
};
struct Output {
    @invariant @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>, @location(1) ambient: vec4<f32>,
    @location(2) light: vec3<f32>, @location(3) eye: vec3<f32>,
    @location(4) extinction: vec3<f32>, @location(5) scatter: vec3<f32>,
    @location(6) world: vec3<f32>,
    @location(7) projected_reflection: vec4<f32>,
    @location(8) normal: vec3<f32>,
};
fn unit(v: vec3<f32>) -> vec3<f32> { return v / max(length(v), 1e-9); }
@vertex
fn vs_main(v: Input) -> Output {
    let model = mat4x4(v.model_0, v.model_1, v.model_2, v.model_3);
    let world = model * vec4(v.position, 1.0);
    let tangent = (model * vec4(v.tangent.xyz, 0.0)).xyz;
    let normal = (model * vec4(v.normal, 0.0)).xyz;
    let binormal = (model * vec4(cross(v.normal, v.tangent.xyz)*v.tangent.w, 0.0)).xyz;
    let eye = unit(globals.camera_pos.xyz-world.xyz);
    let to_light = -globals.light_dir.xyz;
    var out: Output;
    out.clip = globals.view_proj*world;
    out.projected_reflection = vec4(0.5*(out.clip.x+out.clip.w),0.5*(-out.clip.y+out.clip.w),out.clip.z,out.clip.w);
    out.world = world.xyz;
    out.normal = normal;
    out.uv = v.uv+uv_offset.value.xy;
    out.light = unit(vec3(dot(tangent,to_light),dot(binormal,to_light),dot(normal,to_light)));
    out.eye = vec3(dot(tangent,eye),dot(binormal,eye),dot(normal,eye));
    let ambient = select(vec3(0.35), globals.surface_ambient.rgb*globals.fog_color.w, globals.surface_ambient.w>0.5);
    out.ambient = clamp(vec4(ambient*v.color_bgra.zyx,v.color_bgra.a),vec4(0.0),vec4(1.0));
    out.extinction = vec3(1.0);
    out.scatter = vec3(0.0);
    if globals.scatter_rayleigh.w>0.5 {
        let delta = globals.camera_pos.xyz-world.xyz;
        let distance = length(delta);
        let d = clamp(distance*globals.to_sun.w,0.0,2500.0);
        let c = dot(unit(delta),globals.to_sun.xyz);
        let br = globals.scatter_rayleigh.rgb;
        let bm = globals.scatter_mie.rgb;
        let g = globals.scatter_mie.w;
        let sum = br+bm;
        let e = min(exp(-sum*d),vec3(1.0));
        let phase_r = 3.0/(16.0*3.14159265358979)*(1.0+c*c);
        let phase_m = (1.0-g)*(1.0-g)/(4.0*3.14159265358979*pow(max(1.0+g*g-2.0*g*c,1e-6),1.5));
        out.extinction = e;
        out.scatter = clamp((br*phase_r+bm*phase_m)*(vec3(1.0)-e)/max(sum,vec3(1e-9))*globals.surface_diffuse.w,vec3(0.0),vec3(1.0));
    }
    return out;
}
fn shade(in: Output) -> vec4<f32> {
    let tex = textureSampleBias(diffuse_texture,material_sampler,in.uv,globals.fog_range.z);
    let normal_tex = textureSampleBias(normal_texture,material_sampler,in.uv,globals.fog_range.z);
    let spec_tex = textureSampleBias(specular_texture,material_sampler,in.uv,globals.fog_range.z);
    let n = unit(2.0*normal_tex.rgb-vec3(1.0));
    let l = unit(in.light);
    let v = unit(in.eye);
    let d = dot(n,l);
    let reflection = 2.0*d*n-l;
    let highlight = pow(clamp(dot(reflection,v),0.0,1.0),light.specular.w);
    let diffuse_light = select(vec3(0.65),globals.surface_diffuse.rgb,globals.surface_ambient.w>0.5);
    var specular = highlight*light.specular.rgb*spec_tex.rgb;
    var reflected = vec3(0.0);
    if uv_offset.value.z>0.5 {
        specular *= 2.0*light.reflection.y;
        let reflection_uv = in.projected_reflection.xy/in.projected_reflection.w;
        let sample = textureSampleBias(reflection_texture,material_sampler,reflection_uv,globals.fog_range.z);
        reflected = sample.rgb*(2.0*spec_tex.r)*tex.a*in.ambient.a*light.reflection.x;
    }
    let plane = receiver_normal(in.world,in.normal);
    if globals.fog_range.w<0.5 && shadow_data.debug.x>0.5 {
        return vec4<f32>(lighting_debug_colour(in.world,plane,in.clip.xy),1.0);
    }
    var q = 1.0;
    if globals.fog_range.w<0.5 { q = direct_visibility(in.world,plane,in.clip.xy); }
    var rgb = 2.0*(tex.rgb*in.ambient.rgb+(tex.rgb*diffuse_light*clamp(d,0.0,1.0)+specular)*q+reflected)*in.extinction+in.scatter;
    if globals.scatter_rayleigh.w<0.5 {
        let distance = length(in.world-globals.camera_pos.xyz);
        let fog = clamp((distance-globals.fog_range.x)/max(globals.fog_range.y-globals.fog_range.x,0.001),0.0,1.0);
        rgb = mix(rgb,globals.fog_color.rgb,fog);
    }
    if uv_offset.value.z>0.5 { return extended_output(vec4(rgb,1.0)); }
    return extended_output(vec4(rgb,tex.a*in.ambient.a));
}
@fragment fn fs_opaque(in: Output) -> @location(0) vec4<f32> { return vec4(shade(in).rgb,1.0); }
@fragment fn fs_alpha_test(in: Output) -> @location(0) vec4<f32> {
    let color = shade(in);
    if color.a<0.5 { discard; }
    return vec4(color.rgb,1.0);
}
@fragment fn fs_blend(in: Output) -> @location(0) vec4<f32> { return shade(in); }

// HDR-only extension of final RGB; intermediate authored arithmetic remains intact.
fn extended_output(c:vec4<f32>)->vec4<f32> {
    if globals.camera_pos.w>1.5 { return vec4(max(c.rgb,vec3(0.0)),clamp(c.a,0.0,1.0)); }
    return clamp(c,vec4(0.0),vec4(1.0));
}
