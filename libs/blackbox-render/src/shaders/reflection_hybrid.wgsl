// Standard car path: preserve the probe, then optionally add screen-space detail.
fn sharp_environment(world:vec3<f32>,normal:vec3<f32>,direction:vec3<f32>,fallback:vec3<f32>)->vec3<f32> {
    return fallback;
}
fn hybrid_environment(world:vec3<f32>,normal:vec3<f32>,direction:vec3<f32>,fallback:vec3<f32>)->vec3<f32> {
    if material.lighting.x > 0.5 { return fallback; }
    return screen_environment(world,normal,direction,fallback);
}
