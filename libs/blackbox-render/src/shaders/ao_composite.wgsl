// Bilateral upsample and bounded multiplicative composition before particles and UI.
@group(0) @binding(2) var filtered_ao: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((vertex << 1u) & 2u), f32(vertex & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(fragment.xy);
    if params.sampling.w != 0u && fragment.x < params.sizes.x * 0.5 { return vec4<f32>(1.0); }
    let center = position_at(pixel);
    if center.z == 0.0 { return vec4<f32>(1.0); }
    let normal = normal_at(pixel, center);
    let position = (fragment.xy - 0.5) / f32(params.sampling.x) - f32(params.sampling.x / 2u) / f32(params.sampling.x);
    let base = vec2<i32>(floor(position));
    let fraction = fract(position);
    var sum = 0.0;
    var weight_sum = 0.0;
    for (var y = 0; y <= 1; y++) {
        for (var x = 0; x <= 1; x++) {
            let neighbor = base + vec2<i32>(x, y);
            if any(neighbor < vec2<i32>(0)) || any(neighbor >= vec2<i32>(params.sizes.zw)) { continue; }
            let value = textureLoad(filtered_ao, neighbor, 0);
            let bilinear = select(1.0 - fraction, fraction, vec2<i32>(x, y) != vec2<i32>(0));
            let weight = surface_weight(center, normal, neighbor, value) * bilinear.x * bilinear.y;
            sum += value.x * weight;
            weight_sum += weight;
        }
    }
    if weight_sum < 1e-4 { return vec4<f32>(1.0); }
    let occlusion = 1.0 - clamp(sum / weight_sum, 0.0, 1.0);
    // Fog was baked into the scene, so don't turn distant fog into dark silhouettes.
    let fog_visibility = 1.0 - clamp((length(center) - params.fog.x) / max(params.fog.y - params.fog.x, 0.001), 0.0, 1.0);
    let factor = 1.0 - 0.45 * params.effect.y * occlusion * fog_visibility;
    return vec4<f32>(vec3<f32>(factor), 1.0);
}
