// Custom-only surface extension. Independent GGX/Smith/Schlick equations; see custom-paint-response.md.
@group(2) @binding(8) var filtered_environment: texture_cube<f32>;

fn surface_highlight(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, roughness: f32, authored: vec3<f32>) -> vec3<f32> {
    return surface_brdf(n, v, l, roughness, clamp(authored * 0.08, vec3<f32>(0.04), vec3<f32>(0.16)));
}

fn surface_brdf(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, roughness: f32, f0: vec3<f32>) -> vec3<f32> {
    let nl = max(dot(n, l), 0.0);
    let nv = max(dot(n, v), 0.001);
    if nl <= 0.0 { return vec3<f32>(0.0); }
    let h = normalize(v + l);
    let nh = max(dot(n, h), 0.0);
    let vh = max(dot(v, h), 0.0);
    let a = max(roughness * roughness, 0.0064);
    // Angular slope variance is packed once on the CPU, shared by all material layers.
    let a2 = a * a + max(rig.to_light[0].w, 0.0);
    let denominator = nh * nh * (a2 - 1.0) + 1.0;
    let d = a2 / max(PI_F * denominator * denominator, 1e-10);
    let gv = nl * sqrt(nv * nv * (1.0 - a2) + a2);
    let gl = nv * sqrt(nl * nl * (1.0 - a2) + a2);
    let visibility = 0.5 / max(gv + gl, 1e-6);
    let fresnel = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - vh, 5.0);
    return d * visibility * fresnel * nl;
}

fn surface_environment(direction: vec3<f32>, roughness: f32, sharp: vec3<f32>) -> vec3<f32> {
    let level = roughness * f32(textureNumLevels(filtered_environment) - 1u);
    let filtered = textureSampleLevel(filtered_environment, env_sampler, direction, level).rgb;
    return mix(sharp, filtered, smoothstep(0.12, 0.5, roughness));
}

// The legacy scene and captured cube are display-encoded. Physical reflectance factors
// belong in linear light; multiplying an encoded reflection by 0.04 nearly erases it.
fn paint_linear(color: vec3<f32>) -> vec3<f32> {
    let c = max(color, vec3<f32>(0.0));
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}

fn paint_encoded(color: vec3<f32>) -> vec3<f32> {
    let c = max(color, vec3<f32>(0.0));
    return select(1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055, c * 12.92, c <= vec3<f32>(0.0031308));
}

fn surface_diffuse(direct: vec3<f32>, indirect: vec3<f32>, albedo: vec3<f32>, occlusion: f32) -> vec3<f32> {
    // The existing rig/SH sliders supply artistic irradiance intensity. Apply it to
    // linear albedo, with baked occlusion in linear light, rather than crushing
    // already-encoded body colour. Preserve the direct-light result and zero light.
    let bounce = paint_linear(albedo) * max(indirect, vec3<f32>(0.0)) * occlusion;
    return paint_encoded(paint_linear(direct) + bounce);
}

fn rough_paint_color(base: vec3<f32>, environment: vec3<f32>, highlight: vec3<f32>,
    gain: f32, intensity: f32, occlusion: f32, alpha: f32) -> vec3<f32> {
    // Keep the existing diffuse lighting and occlusion, but combine reflected energy
    // in linear light. No ambient floor: a black environment must remain black.
    let reflected = paint_linear(environment) * gain * intensity;
    return paint_encoded(paint_linear(base) + (reflected + highlight) * occlusion * alpha);
}
