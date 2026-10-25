// Vulkan one-bounce geometry reflections. The authored wet material still composites the result.
@group(1) @binding(7) var<uniform> road_control: vec4<f32>;
@group(1) @binding(8) var road_colour: texture_2d<f32>;
@group(1) @binding(9) var road_depth: texture_depth_2d;
@group(1) @binding(10) var road_environment: texture_cube<f32>;
@group(1) @binding(11) var road_sampler: sampler;
@group(1) @binding(12) var road_filtered_environment: texture_cube<f32>;
fn road_environment_at(world:vec3<f32>,normal:vec3<f32>,ray:vec3<f32>,fallback:vec3<f32>,pool:f32,footprint:f32)->vec3<f32> {
    if road_control.x<0.5 || globals.fog_range.w>0.5 || rt_control.scene.x==0u || rt_control.tuning.x<=0.0 { return fallback; }
    let result=trace_environment(world,normal,ray,max(0.03,footprint*0.1),road_control.z);
    if !result.found { return fallback; }
    var radiance=result.radiance;
    let position=result.position;
    // Reuse fully shaded current pixels only when they describe the exact RT hit.
    // This retains decals and material detail, without using screen depth for intersection.
    let clip=globals.view_proj*vec4(position,1.0);
    let uv=clip.xy/max(clip.w,1e-6)*vec2(0.5,-0.5)+0.5;
    if clip.w>0.0 && all(uv>vec2(0.0)) && all(uv<vec2(1.0)) {
        let size=vec2<i32>(textureDimensions(road_depth));
        let depth=textureLoad(road_depth,clamp(vec2<i32>(uv*vec2<f32>(size)),vec2(0),size-1),0);
        if depth>0.0 && abs(road_control.y/depth-clip.w)<max(0.15,clip.w*0.003) {
            radiance=mix(radiance,textureSampleLevel(road_colour,road_sampler,uv,0.0).rgb,rt_control.tuning.y);
        }
    }
    let fade=1.0-smoothstep(road_control.z*(1.0-road_control.w),road_control.z,result.distance);
    return mix(fallback,clamp(radiance,vec3(0.0),vec3(1.0)),fade*rt_control.tuning.x);
}

fn rt_hit_environment(position:vec3<f32>,direction:vec3<f32>,roughness:f32)->vec3<f32> {
    let lod=roughness*f32(textureNumLevels(road_filtered_environment)-1u);
    let sharp=textureSampleLevel(road_environment,road_sampler,direction,0.0).rgb;
    let filtered=textureSampleLevel(road_filtered_environment,road_sampler,direction,lod).rgb;
    return mix(sharp,filtered,smoothstep(0.08,0.32,roughness));
}
