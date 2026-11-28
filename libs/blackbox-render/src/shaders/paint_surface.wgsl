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

// Car paint at night (`shadow_data.car_night`). What the paint mirrors (RT hits, the capture, screen colour) is as dark as
// the night makes it already, so `env_scale` undoes the night dimming its reflected light gets at the end. A clear coat
// reflects only a few percent, so its sharp reflection is lifted by the full level to read as gloss, and a dielectric's
// broad lobe by its square root. Metal flakes and pearl keep their broad lobe as it is (it is their colour); chrome
// takes the square root. Without a local capture the cube is the daytime sky, which the night dims as on the road.
var<private> env_scale: vec3<f32> = vec3<f32>(1.0);
var<private> coat_boost: f32 = 1.0;
// The night blend these boosts follow: zero for materials that keep their authored response.
var<private> paint_night: f32 = 0.0;
// The night lift on the sharp sample (`env_scale` and `coat_boost` together), undone for mirrored car bodies.
var<private> sharp_lift: f32 = 1.0;
fn set_env_scale(kind: f32, enhanced: bool) {
    // Original and unclassified materials (traffic cargo, trims) keep their authored night response.
    let night = select(0.0, saturate(shadow_data.local_options.w), enhanced);
    paint_night = night;
    let lift = mix(1.0, sqrt(max(shadow_data.car_night.x, 0.0)), night);
    let conductor = abs(kind - 5.0) < 0.5 || abs(kind - 11.0) < 0.5;
    let metal = conductor || abs(kind - 3.0) < 0.5 || abs(kind - 4.0) < 0.5;
    env_scale = pow(mix(vec3(1.0), vec3(0.30, 0.35, 0.45), night), vec3(-0.5)) * select(lift, 1.0, metal);
    if abs(local_probe.origin.w) < 0.5 { env_scale *= mix(1.0, 0.12, night); }
    coat_boost = select(lift, select(lift * lift, lift, conductor), metal);
    sharp_lift = select(lift, 1.0, metal) * coat_boost;
}
// The sharp mirror sample: the coat's boost, and bright sources (lamps, lit signs and windows) keep their glow at night.
fn env_lights(c: vec3<f32>) -> vec3<f32> {
    let boost = mix(1.0, shadow_data.car_night.y, paint_night);
    return c * coat_boost * mix(1.0, boost, smoothstep(0.55, 1.0, dot(c, vec3(0.2126, 0.7152, 0.0722))));
}

fn surface_environment(direction: vec3<f32>, roughness: f32, sharp: vec3<f32>) -> vec3<f32> {
    let level = roughness * f32(textureNumLevels(filtered_environment) - 1u);
    let filtered = env_scale * textureSampleLevel(filtered_environment, env_sampler, direction, level).rgb;
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
    let reflected = environment * gain * intensity;
    return paint_encoded(paint_linear(base) + (reflected + highlight) * occlusion * alpha);
}
