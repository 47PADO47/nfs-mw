// Independent coated dielectric/conductor models; see custom-car-materials.md.
@group(2) @binding(9) var linear_environment: texture_cube<f32>;

fn layer_environment(direction: vec3<f32>, roughness: f32, sharp: vec3<f32>) -> vec3<f32> {
    if textureNumLevels(linear_environment) <= 1u { return paint_linear(sharp); }
    let level = roughness * f32(textureNumLevels(linear_environment) - 1u);
    let filtered = textureSampleLevel(linear_environment, env_sampler, direction, level).rgb;
    return mix(paint_linear(sharp), filtered, smoothstep(0.08, 0.32, roughness));
}

fn layer_fresnel(f0: vec3<f32>, nv: f32, roughness: f32) -> vec3<f32> {
    return f0 + (max(vec3<f32>(1.0 - roughness), f0) - f0) * pow(1.0 - nv, 5.0);
}

// Stable UV cells; subpixel cells converge to their mean instead of sparkling in motion.
fn flake_variation(uv: vec2<f32>) -> f32 {
    let cells = uv * 512.0;
    let footprint = max(length(dpdx(cells)), length(dpdy(cells)));
    let cell = floor(cells);
    let hash = fract(sin(dot(cell, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    return 1.0 + (hash - 0.5) * 0.3 * (1.0 - smoothstep(0.35, 1.0, footprint));
}

fn layered_material(n: vec3<f32>, v: vec3<f32>, sun: vec3<f32>, base: vec3<f32>,
    albedo: vec3<f32>, opacity: f32, visibility: f32, occlusion: f32,
    uv: vec2<f32>, sharp: vec3<f32>, world: vec3<f32>) -> vec4<f32> {
    let kind = u32(material.surface.w + 0.1);
    let roughness = material.surface.x;
    let nv = saturate(dot(n, v));
    let reflected = reflect(-v, n);
    let variation = flake_variation(uv);
    var environment = layer_environment(reflected, roughness, sharp);
    if roughness < 0.4 && kind != 6u {
        let screen = hybrid_environment(world, n, reflected, paint_encoded(environment));
        environment = mix(environment, paint_linear(screen), 1.0 - smoothstep(0.15, 0.4, roughness));
    }
    if kind == 10u { return vec4<f32>(base, opacity); }
    var coat = 0.0;
    if shadow_data.options.w > 0.5 { coat = material.surface.y; }
    var f0 = vec3<f32>(mix(0.04, 0.015, coat));
    var body = paint_linear(base);
    if kind == 5u {
        f0 = clamp(mix(vec3<f32>(0.65), paint_linear(albedo), 0.35), vec3<f32>(0.06), vec3<f32>(0.95));
        body = vec3<f32>(0.0);
    }
    if kind == 9u { f0 = vec3<f32>(0.025); }
    let fresnel = layer_fresnel(f0, nv, roughness);
    let highlight = visibility * rig.color[0].xyz * surface_brdf(n, v, sun, roughness, f0);
    let reflection = environment * fresnel * material.surface.z;
    if kind == 6u {
        // Existing tint/opacity controls absorption. Reflections grow towards grazing
        // angles, independently of tint, with no second clearcoat on the glass.
        let glass_f = 0.04 + 0.96 * pow(1.0 - nv, 5.0);
        let alpha = clamp(opacity + (1.0 - opacity) * glass_f, 0.0, 1.0);
        let reflected_rgb = paint_encoded((environment * glass_f * material.surface.z + highlight) * occlusion);
        let transmitted_rgb = paint_encoded(body * (1.0 - glass_f));
        return vec4<f32>((transmitted_rgb * opacity + reflected_rgb) / max(alpha, 0.01), alpha);
    }
    let dielectric_loss = 1.0 - max(fresnel.r, max(fresnel.g, fresnel.b));
    var color = body * dielectric_loss + (reflection + highlight) * occlusion;
    if kind == 3u || kind == 4u {
        let pigment = sqrt(clamp(paint_linear(albedo), vec3<f32>(0.0), vec3<f32>(1.0)));
        var flake_color = mix(vec3<f32>(0.10), pigment * 0.35, 0.65);
        var flake_strength = 0.30 * variation;
        if kind == 4u {
            // Retain the installed pearl facing-colour ramp; add a broad tinted lobe.
            flake_color = mix(vec3<f32>(0.08), pigment, 0.25);
            flake_strength = 0.35 + 0.25 * pow(1.0 - nv, 2.0);
        }
        let flake_roughness = max(roughness, 0.36);
        let flake_env = layer_environment(reflected, flake_roughness, sharp);
        let flake_sun = visibility * rig.color[0].xyz * surface_brdf(n, v, sun, flake_roughness, flake_color);
        color += flake_strength * occlusion * (flake_env * layer_fresnel(flake_color, nv, flake_roughness)
            * material.surface.z + flake_sun);
    }
    if coat > 0.0 {
        let coat_roughness = select(0.12, 0.18, kind == 7u || kind == 8u);
        let coat_f = layer_fresnel(vec3<f32>(0.04), nv, coat_roughness) * coat;
        let coat_env = layer_environment(reflected, coat_roughness, sharp);
        let coat_sun = visibility * rig.color[0].xyz * surface_brdf(n, v, sun, coat_roughness, vec3<f32>(0.04));
        color = color * (vec3<f32>(1.0) - coat_f) + occlusion
            * (coat_env * coat_f * material.surface.z + coat_sun * coat);
    }
    return vec4<f32>(paint_encoded(color), opacity);
}
