// Tangent-space scenery; docs/specs/world-normal-materials.md.
// Set only for planar-reflection captures: discard fragments behind `globals.clip_plane`. Left off elsewhere so
// the opaque draws of the main view keep early depth testing.
override CLIP_PLANE: bool = false;
struct Globals {
    view_proj: mat4x4<f32>, camera_pos: vec4<f32>, light_dir: vec4<f32>,
    fog_color: vec4<f32>, fog_range: vec4<f32>, scatter_rayleigh: vec4<f32>,
    scatter_mie: vec4<f32>, to_sun: vec4<f32>, surface_ambient: vec4<f32>, surface_diffuse: vec4<f32>,
    camera_right: vec4<f32>, clip_plane: vec4<f32>,
};
struct Offset { value: vec4<f32> };
struct Light { specular: vec4<f32>, reflection: vec4<f32>, water: vec4<f32>, wet_tuning: vec4<f32> };
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
    @location(8) tangent: vec4<f32>,
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
    @location(0) uv: vec2<f32>, @location(1) ambient: vec4<f32>,
    @location(2) light: vec3<f32>, @location(3) eye: vec3<f32>,
    @location(4) extinction: vec3<f32>, @location(5) scatter: vec3<f32>,
    @location(6) world: vec3<f32>,
    @location(7) projected_reflection: vec4<f32>,
    @location(8) normal: vec3<f32>,
    @location(9) tangent: vec3<f32>, @location(10) binormal: vec3<f32>,
    @location(11) @interpolate(flat) receives_shadow: u32,
};
fn unit(v: vec3<f32>) -> vec3<f32> { return v / max(length(v), 1e-9); }
@vertex
fn vs_main(v: Input, placed: InstanceIn) -> Output {
    let model = instance_model(placed);
    let world = model * vec4(v.position, 1.0);
    let tangent = (model * vec4(v.tangent.xyz, 0.0)).xyz;
    let normal = (model * vec4(v.normal, 0.0)).xyz;
    let binormal = (model * vec4(cross(v.normal, v.tangent.xyz)*v.tangent.w, 0.0)).xyz;
    let eye = unit(globals.camera_pos.xyz-world.xyz);
    let to_light = -globals.light_dir.xyz;
    var out: Output;
    out.receives_shadow = instance_receives_shadow(placed);
    out.clip = globals.view_proj*world;
    out.projected_reflection = vec4(0.5*(out.clip.x+out.clip.w),0.5*(-out.clip.y+out.clip.w),out.clip.z,out.clip.w);
    out.world = world.xyz;
    out.normal = normal;
    out.tangent = tangent; out.binormal = binormal;
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
fn shade(in: Output, receive_ao: bool) -> vec4<f32> {
    if CLIP_PLANE && dot(globals.clip_plane, vec4<f32>(in.world, 1.0)) < 0.0 { discard; }
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
        // Enhanced environment shading replaces the legacy sample completely.
        if !(light.reflection.z>0.5 && uv_offset.value.w>0.5 && light.water.x>0.0) {
            let sample = textureSampleBias(reflection_texture,material_sampler,reflection_uv,globals.fog_range.z);
            reflected = sample.rgb*(2.0*spec_tex.r)*tex.a*in.ambient.a*light.reflection.x;
        }
    }
    let plane = receiver_normal(in.world,in.normal);
    if globals.fog_range.w<0.5 && shadow_data.debug.x>0.5 {
        return vec4<f32>(lighting_debug_colour(in.world,plane,in.clip.xy),1.0);
    }
    var q = 1.0;
    if globals.fog_range.w<0.5 && in.receives_shadow != 0u { q = direct_visibility(in.world,plane,in.clip.xy); }
    if globals.fog_range.w>2.5 && in.receives_shadow != 0u { q = capture_sun_visibility(in.world,plane); }
    var ao = 1.0;
    q *= cloud_visibility(in.world);
    if receive_ao && globals.fog_range.w<0.5 { ao = ambient_visibility(in.clip.xy); }
    let shading_normal = unit(in.tangent*n.x+in.binormal*n.y+in.normal*n.z);
    let ambient = in.ambient.rgb*world_environment(shading_normal);
    var surface = 2.0*(tex.rgb*ambient*ao+(tex.rgb*diffuse_light*clamp(d,0.0,1.0)+specular)*q+reflected);
    if uv_offset.value.z>0.5 && light.wet_tuning.x>0.5 && light.water.x>0.0 {
        surface = wet_surface(in, tex, spec_tex, n, l, v, diffuse_light, q, ao, surface);
    }
    var rgb = surface*in.extinction+in.scatter;
    if globals.scatter_rayleigh.w<0.5 {
        let distance = length(in.world-globals.camera_pos.xyz);
        let fog = clamp((distance-globals.fog_range.x)/max(globals.fog_range.y-globals.fog_range.x,0.001),0.0,1.0);
        rgb = mix(rgb,globals.fog_color.rgb,fog);
    }
    if uv_offset.value.z>0.5 { return extended_output(vec4(rgb,1.0)); }
    return extended_output(vec4(rgb,tex.a*in.ambient.a));
}
@fragment fn fs_opaque(in: Output) -> @location(0) vec4<f32> { return vec4(shade(in,true).rgb,1.0); }
@fragment fn fs_alpha_test(in: Output) -> @location(0) vec4<f32> {
    let color = shade(in,true);
    if color.a<0.5 { discard; }
    return vec4(color.rgb,1.0);
}
@fragment fn fs_blend(in: Output) -> @location(0) vec4<f32> { return shade(in,false); }

// HDR-only extension of final RGB; intermediate authored arithmetic remains intact.
fn extended_output(c:vec4<f32>)->vec4<f32> {
    if globals.camera_pos.w>1.5 { return vec4(max(c.rgb,vec3(0.0)),clamp(c.a,0.0,1.0)); }
    return clamp(c,vec4(0.0),vec4(1.0));
}

// Independent enhanced response. Existing authored masks remain reflection eligibility, not water depth.
fn wet_hash(p:vec2<i32>)->f32 {
    var h = bitcast<u32>(p.x)*1597334677u ^ bitcast<u32>(p.y)*3812015801u;
    h = (h ^ (h>>16u))*2246822519u;
    return f32((h ^ (h>>13u)) & 65535u)/65535.0;
}
fn wet_noise(p:vec2<f32>)->f32 {
    let cell = vec2<i32>(floor(p));
    let f = fract(p);
    let t = f*f*(vec2(3.0)-2.0*f);
    return mix(mix(wet_hash(cell),wet_hash(cell+vec2(1,0)),t.x),
        mix(wet_hash(cell+vec2(0,1)),wet_hash(cell+vec2(1,1)),t.x),t.y);
}
fn pooled_coverage(world:vec2<f32>)->f32 {
    if light.wet_tuning.y<=0.0 { return 0.0; }
    let p = world/max(light.wet_tuning.z,0.25);
    let field = 0.7*wet_noise(p)+0.3*wet_noise(p*3.7+vec2(19.0));
    let threshold = 0.86-0.48*light.water.y*light.wet_tuning.y;
    // Widen edges when patches fall below a pixel; no screen-anchored dither or temporal history.
    let width = max(0.025,fwidth(field));
    return smoothstep(threshold-width,threshold+width,field)*light.water.y;
}
fn wet_linear(c:vec3<f32>)->vec3<f32> { return pow(max(c,vec3(0.0)),vec3(2.2)); }
fn wet_encoded(c:vec3<f32>)->vec3<f32> { return pow(max(c,vec3(0.0)),vec3(1.0/2.2)); }
fn wet_capture(uv:vec2<f32>,pixel:vec2<f32>)->vec3<f32> {
    return textureSampleLevel(reflection_texture,material_sampler,clamp(uv,pixel*0.5,vec2(1.0)-pixel*0.5),0.0).rgb;
}
fn wet_surface(in:Output,tex:vec4<f32>,spec_tex:vec4<f32>,n:vec3<f32>,l:vec3<f32>,
    v:vec3<f32>,diffuse_light:vec3<f32>,q:f32,ao:f32,dry:vec3<f32>)->vec3<f32> {
    // Only upward authored reflective receivers. Vertical walls never acquire procedural pools.
    let slope = smoothstep(0.90,0.995,unit(in.normal).z);
    let damp = light.water.x*slope;
    let ray_footprint = max(length(dpdx(in.world)),length(dpdy(in.world)));
    let pool = pooled_coverage(in.world.xy)*slope;
    let p = in.world.xy;
    let footprint = max(length(dpdx(p)),length(dpdy(p)));
    // Finish derivatives before a pixel-dependent return. Vertical receivers are dry.
    if damp <= 0.0 { return dry; }
    let ripple_fade = 1.0-smoothstep(0.12,0.8,footprint);
    let phase = light.water.z*0.785398163;
    let ripples = vec2(sin(dot(p,vec2(7.3,4.1))+phase),cos(dot(p,vec2(-3.7,8.9))-phase*2.0));
    let ripple = ripples*(0.012*light.wet_tuning.w*light.water.w*pool*ripple_fade);
    // Water smooths the road micro-normal, but keeps its geometric slope.
    let water_n = unit(mix(n,vec3(ripple,1.0),pool*0.94));
    let nv = clamp(dot(water_n,v),0.0,1.0);
    let fresnel = 0.02+0.98*pow(1.0-nv,5.0);
    let mask = clamp(2.0*spec_tex.r*tex.a*in.ambient.a,0.0,1.0);
    // The legacy mask already attenuates the old nonphysical reflection. Keep its zero regions,
    // but let a continuous water layer approach dielectric reflectance within eligible regions.
    let water_mask = mix(mask,smoothstep(0.0,0.08,mask),pool);
    let reflectance = fresnel*mix(0.3,1.0,pool)*water_mask;
    // Receiver pixel size for the road march's origin bias. Derivatives must stay in uniform control flow.
    let world_footprint = ray_footprint;
    var filtered = vec3(0.0);
    let road_environment_path = light.reflection.z > 0.5 && uv_offset.value.w>0.5;
    if road_environment_path && reflectance > 0.0 {
        let world_n = unit(in.tangent*water_n.x+in.binormal*water_n.y+in.normal*water_n.z);
        let ray = reflect(-unit(globals.camera_pos.xyz-in.world),world_n);
        // Resolve the fallback only for misses or partial traced contributions.
        filtered = road_environment_at(in.world,world_n,ray,pool,world_footprint);
    }
    // The capture taps only serve the paths that do not trace the road environment.
    if !road_environment_path && reflectance > 0.0 {
        let uv = in.projected_reflection.xy/in.projected_reflection.w;
        let pixel = vec2(1.0)/vec2<f32>(textureDimensions(reflection_texture));
        let radius = mix(3.5,0.35,pool);
        let center = wet_capture(uv,pixel);
        // Spatially filtered film; pooled water retains the capture's detail. No fabricated parallax offsets.
        filtered = (wet_capture(uv+pixel*vec2(radius,0.0),pixel)
            +wet_capture(uv-pixel*vec2(radius,0.0),pixel)
            +wet_capture(uv+pixel*vec2(0.0,radius),pixel)
            +wet_capture(uv-pixel*vec2(0.0,radius),pixel)+center*4.0)/8.0;
    }
    let shading_normal = unit(in.tangent*water_n.x+in.binormal*water_n.y+in.normal*water_n.z);
    let base = 2.0*tex.rgb*(in.ambient.rgb*world_environment(shading_normal)*ao+diffuse_light*max(dot(water_n,l),0.0)*q);
    let diffuse = wet_linear(base)*mix(1.0,0.72,damp)*mix(1.0,0.78,pool)*(1.0-reflectance);
    let power = mix(24.0,192.0,pool);
    let h = unit(l+v);
    let glint = pow(max(dot(water_n,h),0.0),power)*(power+2.0)/50.0;
    let spec = wet_linear(light.specular.rgb)*glint*q*fresnel*water_mask;
    let env = wet_linear(filtered)*reflectance*select(0.0,1.0,light.reflection.x>0.0);
    return mix(dry,wet_encoded(diffuse+spec+env),damp);
}
