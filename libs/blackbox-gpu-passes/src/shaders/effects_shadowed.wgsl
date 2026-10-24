
// Shadow reception on, for a build that has `shadow_receiver.wgsl` in front of `effects.wgsl` (the receiver at
// bind group 1). A streak or glow vertex with `detail.y` set takes the sun's shadow: its colour falls to a third in
// full shadow, so a spark or wind trail stays visible in the dark. The reduced rear-view mirror view takes none.
fn receive_shadow(world: vec3<f32>, clip: vec4<f32>, flag: f32) -> f32 {
    if flag < 0.5 || globals.fog_range.w > 0.5 {
        return 1.0;
    }
    let visible = direct_visibility(world, shadow_data.sun.xyz, clip.xy);
    return mix(0.33, 1.0, visible);
}
