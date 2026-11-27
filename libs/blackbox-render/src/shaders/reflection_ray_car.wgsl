// One geometry ray for sharp car reflections. Rough lobes retain their filtered probe.
fn sharp_environment(world:vec3<f32>,normal:vec3<f32>,direction:vec3<f32>,fallback:vec3<f32>)->vec3<f32> {
    if material.lighting.x > 0.5 { return fallback; }
    // Preserve the reconstructed Original PC response and its optional SSR setting.
    if material.specular_min.w>0.5 { return fallback; }
    if globals.fog_range.w>0.5 || rt_control.scene.x==0u || rt_control.tuning.z<=0.0 { return fallback; }
    let ray=normalize(direction);
    let result=trace_environment(world,normal,ray,0.015,rt_control.tuning.w);
    if !result.found { return fallback; }
    // Consistent hit shading on and off screen avoids a visibility-dependent lighting switch. At night a mirrored lamp
    // or lit window may exceed white, so it can bloom.
    let fade=1.0-smoothstep(rt_control.tuning.w*0.65,rt_control.tuning.w,result.distance);
    let ceiling=mix(1.0,4.0,saturate(shadow_data.local_options.w));
    return mix(fallback,clamp(result.radiance,vec3(0.0),vec3(ceiling)),fade*rt_control.tuning.z);
}

// A hit lit as the world is: the night tint over the sun and ambient light, then the local lamps on the albedo
// (`scene.wgsl`). Emission is added after this, untinted.
fn rt_hit_lit(lit:vec3<f32>,albedo:vec3<f32>,position:vec3<f32>,n:vec3<f32>)->vec3<f32> {
    if shadow_data.car_night.z<0.5 { return lit; }
    return lit*mix(vec3(1.0),vec3(0.30,0.35,0.45),shadow_data.local_options.w)+albedo*local_lighting(position,n);
}

// A mirrored car body (the player's own, traffic, cargo) is shaded as it looks on screen, so the night lift meant for the
// dim world must not brighten it again: otherwise a spoiler or a cargo drum mirrors brighter than the panel itself.
fn rt_glossy_hit_scale()->f32 { return select(1.0,1.0/max(sharp_lift,1e-3),shadow_data.car_night.z>0.5); }

// Sharp geometry was evaluated once before the material's roughness filtering.
fn hybrid_environment(world:vec3<f32>,normal:vec3<f32>,direction:vec3<f32>,fallback:vec3<f32>)->vec3<f32> {
    if material.lighting.x > 0.5 { return fallback; }
    if material.specular_min.w>0.5 { return screen_environment(world,normal,direction,fallback); }
    return fallback;
}

// Secondary glossy hits use the existing probe rather than recursively tracing a mirror.
// Cars keep their independent hit shading; road redraws can reuse exact current pixels.
fn rt_screen_radiance(position:vec3<f32>)->vec4<f32> { return vec4(0.0); }
// The hit's own reflection, dimmed as the raster dims it at night (the receiver's `env_scale` is not applied here: the
// receiver scales the whole hit once). Without this a mirrored glossy surface glows brighter than the surface itself.
fn rt_hit_environment(position:vec3<f32>,direction:vec3<f32>,roughness:f32)->vec3<f32> {
    let ray=probe_direction(position,direction,roughness);
    let sharp=textureSampleLevel(env_map,env_sampler,ray,0.0).rgb;
    let level=roughness*f32(textureNumLevels(filtered_environment)-1u);
    let filtered=textureSampleLevel(filtered_environment,env_sampler,ray,level).rgb;
    let env=mix(sharp,filtered,smoothstep(0.12,0.5,roughness));
    if shadow_data.car_night.z<0.5 { return env; }
    let night=saturate(shadow_data.local_options.w);
    let dim=select(1.0,mix(1.0,0.12,night),abs(local_probe.origin.w)<0.5);
    return env*sqrt(mix(vec3(1.0),vec3(0.30,0.35,0.45),night))*dim;
}
