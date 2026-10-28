// One geometry ray for sharp car reflections. Rough lobes retain their filtered probe.
fn sharp_environment(world:vec3<f32>,normal:vec3<f32>,direction:vec3<f32>,fallback:vec3<f32>)->vec3<f32> {
    if material.lighting.x > 0.5 { return fallback; }
    // Preserve the reconstructed Original PC response and its optional SSR setting.
    if material.specular_min.w>0.5 { return fallback; }
    if globals.fog_range.w>0.5 || rt_control.scene.x==0u || rt_control.tuning.z<=0.0 { return fallback; }
    let ray=normalize(direction);
    let result=trace_environment(world,normal,ray,0.015,rt_control.tuning.w);
    if !result.found { return fallback; }
    // Consistent hit shading on and off screen avoids a visibility-dependent lighting switch.
    let fade=1.0-smoothstep(rt_control.tuning.w*0.65,rt_control.tuning.w,result.distance);
    return mix(fallback,clamp(result.radiance,vec3(0.0),vec3(1.0)),fade*rt_control.tuning.z);
}

// Sharp geometry was evaluated once before the material's roughness filtering.
fn hybrid_environment(world:vec3<f32>,normal:vec3<f32>,direction:vec3<f32>,fallback:vec3<f32>)->vec3<f32> {
    if material.lighting.x > 0.5 { return fallback; }
    if material.specular_min.w>0.5 { return screen_environment(world,normal,direction,fallback); }
    return fallback;
}

// Secondary glossy hits use the existing probe rather than recursively tracing a mirror.
// Cars keep their independent hit shading; road redraws can reuse exact current pixels.
fn rt_screen_radiance(position:vec3<f32>)->vec4<f32> { return vec4(0.0); }
fn rt_hit_environment(position:vec3<f32>,direction:vec3<f32>,roughness:f32)->vec3<f32> {
    let ray=probe_direction(position,direction,roughness);
    let sharp=textureSampleLevel(env_map,env_sampler,ray,0.0).rgb;
    return surface_environment(ray,roughness,sharp);
}
