
// Shadow reception off: the streaks and glows take no shadow. `effects_shadowed.wgsl` takes this file's place in the
// renderer's build that has the sun's shadow map, with the shadow receiver in front.
fn receive_shadow(world: vec3<f32>, clip: vec4<f32>, flag: f32) -> f32 {
    return 1.0;
}
