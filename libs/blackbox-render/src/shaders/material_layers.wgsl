// Independent coated dielectric/conductor models; see custom-car-materials.md.
@group(2) @binding(9) var linear_environment: texture_cube<f32>;

fn layer_environment(direction: vec3<f32>, roughness: f32, sharp: vec3<f32>) -> vec3<f32> {
    if textureNumLevels(linear_environment) <= 1u { return paint_linear(sharp); }
    let level = roughness * f32(textureNumLevels(linear_environment) - 1u);
    let filtered = pow(env_scale, vec3(2.2)) * textureSampleLevel(linear_environment, env_sampler, direction, level).rgb;
    return mix(paint_linear(sharp), filtered, smoothstep(0.08, 0.32, roughness));
}

fn layer_fresnel(f0: vec3<f32>, nv: f32, roughness: f32) -> vec3<f32> {
    return f0 + (max(vec3<f32>(1.0 - roughness), f0) - f0) * pow(1.0 - nv, 5.0);
}

// Four neighbouring lattice samples, one XY face of a volume of metallic pigment.
fn flake_samples(seeds: vec4<u32>) -> vec4<f32> {
    var h = (seeds ^ (seeds >> vec4<u32>(16u))) * 2246822519u;
    h ^= h >> vec4<u32>(13u);
    return vec4<f32>(h & vec4<u32>(65535u)) / 32767.5 - 1.0;
}

const FLAKE_AXIS = vec3<u32>(1597334677u, 3812015801u, 2798796415u);
const FLAKE_FACE = vec4<u32>(0u, 1597334677u, 3812015801u, 1114383182u);

// Interpolating a full volume avoids the periodic bands left by slicing layers of isolated dots.
fn flake_variation(local: vec3<f32>) -> f32 {
    let cells = local * 640.0;
    let footprint = max(length(dpdx(cells)), length(dpdy(cells)));
    let resolved = 1.0 - smoothstep(0.25, 1.15, footprint);
    if resolved <= 0.0 { return 1.0; }
    let index = bitcast<vec3<u32>>(vec3<i32>(floor(cells))) * FLAKE_AXIS;
    let xy = vec4<u32>(index.x + index.y + index.z) + FLAKE_FACE;
    let t = fract(cells);
    let w = t * t * (3.0 - 2.0 * t);
    let face = mix(flake_samples(xy), flake_samples(xy + vec4<u32>(FLAKE_AXIS.z)), w.z);
    let rows = mix(face.xz, face.yw, w.x);
    let noise = mix(rows.x, rows.y, w.y);
    return 1.0 + 0.65 * resolved * noise * abs(noise);
}

// The XY slope and coverage of smoothly rounded drops at one physical scale.
fn bead_field(panel: vec2<f32>, frequency: f32, occupancy: f32) -> vec3<f32> {
    // Bend and stagger the sample rows; vary occupancy over patches of the surface.
    let bend = sin(panel.yx * vec2<f32>(49.0, 61.0) + sin(panel * 29.0));
    var grid = (panel + 0.005 * bend) * frequency;
    let footprint = max(length(dpdx(grid)), length(dpdy(grid)));
    grid.x += fract(floor(grid.y) * 0.618034) * 5.0;
    let index = bitcast<vec2<u32>>(vec2<i32>(floor(grid)));
    let random = flake_samples(vec4<u32>(index.x * FLAKE_AXIS.x + index.y * FLAKE_AXIS.y) + FLAKE_FACE);
    let radius = 0.145 + 0.12 * random.z;
    let delta = (fract(grid) - (0.5 + (0.48 - radius) * random.xy)) / radius;
    let cap = max(0.0, 1.0 - dot(delta, delta));
    let resolved = 1.0 - smoothstep(0.50, 1.8, footprint / radius);
    let weight = resolved * step(random.w, occupancy * 1.7 - 1.05 + 0.38 * bend.x * bend.y);
    return vec3<f32>(-1.8 * delta * cap, smoothstep(0.0, 0.42, cap)) * weight;
}

// The same curved centreline defines the head and the thin wake behind it.
fn trail_offset(distance: f32, seed: f32) -> f32 {
    return 0.034 * sin(distance * 15.0 + seed) + 0.013 * sin(distance * 33.0 - seed);
}

fn running_drop(panel: vec2<f32>, flow: vec2<f32>) -> vec4<f32> {
    var grid = panel * 7.25;
    let footprint = max(length(dpdx(grid)), length(dpdy(grid)));
    grid.x += fract(floor(grid.y) * 0.754878) * 7.0;
    let cell = bitcast<vec2<u32>>(vec2<i32>(floor(grid)));
    let noise = flake_samples(vec4<u32>(cell.x * FLAKE_AXIS.x + cell.y * FLAKE_AXIS.y) + FLAKE_FACE);
    let speed = length(flow);
    let along = flow / max(speed, 0.001);
    let across = vec2<f32>(-along.y, along.x);
    let rate = (6.0 + round(3.0 * noise.y)) / 16.0;
    let phase = fract(globals.water.y * rate + noise.x);
    let progress = 0.61 * smoothstep(0.10, 0.46, phase) + 0.39 * smoothstep(0.54, 0.78, phase);
    let travel = 0.72 * smoothstep(0.01, 0.55, speed);
    let head = (progress - 0.5) * travel;
    let delta = fract(grid) - (0.5 + 0.055 * noise.zw);
    let longitudinal = dot(delta, along);
    let lateral = dot(delta, across) - trail_offset(longitudinal, noise.z * 6.28);
    let radius = 0.035 + 0.015 * noise.w;
    let shape = vec2<f32>(lateral / radius, (longitudinal - head) / (1.4 * radius));
    let cap = max(0.0, 1.0 - dot(shape, shape));
    let behind = head - longitudinal;
    let wake_length = max(0.001, min(progress * travel, 0.23));
    let wake = smoothstep(0.0, radius, behind) * (1.0 - smoothstep(0.0, wake_length, behind));
    let width = radius * (0.26 + 0.44 * wake);
    let ribbon = wake * max(0.0, 1.0 - lateral*lateral/(width*width));
    let resolved = 1.0 - smoothstep(radius*0.45, radius*2.2, footprint);
    let weight = resolved * smoothstep(0.0,0.09,phase) * (1.0-smoothstep(0.80,1.0,phase))
        * smoothstep(0.01,0.20,speed);
    let tilt = -1.9 * cap * (across*shape.x + along*shape.y) - across*(lateral/width)*ribbon;
    return vec4<f32>(tilt, max(cap, 0.55*ribbon), ribbon) * weight;
}

fn beaded_surface(n: vec3<f32>, world: vec3<f32>, local: vec3<f32>, wet: f32, force: vec3<f32>) -> vec4<f32> {
    let area = cross(dpdx(local), dpdy(local));
    let normal = area / max(length(area), 1e-12);
    let flow = force - normal * dot(force, normal);
    var panel = local.xy;
    var drift = flow.xy;
    if abs(normal.x) > max(abs(normal.y), abs(normal.z)) {
        panel = local.yz; drift = flow.yz;
    } else if abs(normal.y) > abs(normal.z) {
        panel = local.xz; drift = flow.xz;
    }
    let fine = bead_field(panel, 124.0, 0.79);
    let broad = bead_field(vec2<f32>(0.8*panel.x - 0.6*panel.y, 0.6*panel.x + 0.8*panel.y), 37.0, 0.34);
    // Chart projection foreshortens the downhill vector. Preserve its tangent speed.
    let projected_flow = drift * (length(flow) / max(length(drift), 0.001));
    let runner = running_drop(panel, projected_flow);
    let pinned = fine.xy + vec2<f32>(0.8*broad.x + 0.6*broad.y, -0.6*broad.x + 0.8*broad.y);
    let slope = pinned * (1.0 - runner.w) + runner.xy;
    let dx = dpdx(world); let dy = dpdy(world);
    let u = cross(dy,n); let v = cross(n,dx);
    let det = dot(dx,u);
    let tilt = (dot(slope,dpdx(panel))*u + dot(slope,dpdy(panel))*v) * sign(det) / max(abs(det),1e-12);
    let top = smoothstep(-0.65, 0.20, n.z);
    return vec4<f32>(normalize(n - tilt * wet * top), max(max(fine.z, broad.z) * (1.0 - runner.w), runner.z) * top);
}

fn layered_material(n: vec3<f32>, base_nv: f32, v: vec3<f32>, sun: vec3<f32>, base: vec3<f32>,
    albedo: vec3<f32>, opacity: f32, visibility: f32, occlusion: f32, specular_ao: f32,
    local: vec3<f32>, sharp: vec3<f32>, world: vec3<f32>, coat_ray: vec3<f32>, coat_width: f32) -> vec4<f32> {
    // Every environment term below scales with this; highlights do not.
    let intensity = material.surface.z * specular_ao;
    let kind = u32(material.surface.w + 0.1);
    if kind == 10u { return vec4<f32>(base, opacity); }
    let roughness = material.surface.x;
    let nv = saturate(dot(n, v));
    var reflected = reflect(-v, n);
    if material.lighting.x < 0.5 { reflected = probe_direction(world, reflected, roughness); }
    var variation = 1.0;
    if kind == 3u { variation = flake_variation(local); }
    var environment = layer_environment(reflected, roughness, sharp);
    if roughness < 0.4 && kind != 6u {
        let screen = hybrid_environment(world, n, reflected, paint_encoded(environment));
        environment = mix(environment, paint_linear(screen), 1.0 - smoothstep(0.15, 0.4, roughness));
    }
    var coat = 0.0;
    if shadow_data.options.w > 0.5 { coat = material.surface.y; }
    var f0 = vec3<f32>(mix(0.04, 0.015, coat));
    var body = paint_linear(base);
    if kind == 5u || kind == 11u {
        // Body chrome uses the composited paint/vinyl colour with stronger conductor tint.
        // Bare metal keeps its authored neutral response and has no diffuse paint layer.
        let tint = select(0.35, 0.60, kind == 11u);
        f0 = clamp(mix(vec3<f32>(0.65), paint_linear(albedo), tint), vec3<f32>(0.06), vec3<f32>(0.95));
        body = vec3<f32>(0.0);
    }
    if kind == 9u { f0 = vec3<f32>(0.025); }
    if kind == 8u {
        // Powder-coated alloy retains a broad tinted return even outside direct sunlight.
        let pigment = sqrt(clamp(paint_linear(albedo), vec3<f32>(0.0), vec3<f32>(1.0)));
        f0 = mix(vec3<f32>(0.13), pigment, 0.32);
        body *= 0.83;
    }
    let fresnel = layer_fresnel(f0, nv, roughness);
    let highlight = visibility * rig.color[0].xyz * surface_brdf(n, v, sun, roughness, f0);
    let reflection = environment * fresnel * intensity;
    if kind == 6u {
        // Existing tint/opacity controls absorption. Reflections grow towards grazing
        // angles, independently of tint, with no second clearcoat on the glass.
        let glass_f = 0.04 + 0.96 * pow(1.0 - nv, 5.0);
        let pane_f = 0.04 + 0.96 * pow(1.0 - base_nv, 5.0);
        let alpha = clamp(opacity + (1.0 - opacity) * pane_f, 0.0, 1.0);
        // Bound the unresolved solar disc in the display-referred glass pass.
        let solar = highlight / (1.0 + max(highlight.r, max(highlight.g, highlight.b)));
        let reflected_rgb = paint_encoded((environment * glass_f * intensity + solar) * occlusion);
        let transmitted_rgb = paint_encoded(min(body, paint_linear(albedo) * 0.20) * (1.0 - pane_f));
        return vec4<f32>((transmitted_rgb * opacity + reflected_rgb) / max(alpha, 0.01), alpha);
    }
    let dielectric_loss = 1.0 - max(fresnel.r, max(fresnel.g, fresnel.b));
    var grain = 1.0;
    if kind == 8u { grain = mix(flake_variation(local * 0.62), flake_variation(local * 1.30), 0.50); }
    var color = body * dielectric_loss + (reflection * mix(1.0, grain, 0.72) + highlight * grain) * occlusion;
    if kind == 3u || kind == 4u {
        let pigment = sqrt(clamp(paint_linear(albedo), vec3<f32>(0.0), vec3<f32>(1.0)));
        var flake_color = mix(vec3<f32>(0.48), pigment, 0.70);
        var flake_strength = 0.55;
        // Flakes displace part of the diffuse pigment response beneath the clearcoat.
        if kind == 3u {
            // Pigment filters the neutral flake return below the coat. Keep the strongest
            // channel and achromatic paints at their existing reflectance.
            let peak = max(max(pigment.r, pigment.g), pigment.b);
            let tint = max(pigment, vec3<f32>(0.025)) / max(peak, 0.025);
            // In shade the broader return comes primarily from the environment; retain the
            // pigment hue there while a directly lit flake still catches neutral sunlight.
            let direct_share = saturate(dot(n, sun)) * visibility;
            let silver = mix(vec3<f32>(0.48), 0.48 * tint, mix(0.64, 0.50, direct_share));
            flake_color = mix(silver, pigment, 0.70);
            color -= body * dielectric_loss * flake_strength * flake_color;
        }
        if kind == 4u {
            // Retain the installed pearl facing-colour ramp; add a broad tinted lobe.
            flake_color = mix(vec3<f32>(0.08), pigment, 0.25);
            flake_strength = 0.35 + 0.25 * pow(1.0 - nv, 2.0);
        }
        let flake_roughness = max(roughness, select(0.36, 0.45, kind == 3u));
        let flake_env = layer_environment(reflected, flake_roughness, sharp);
        let flake_sun = visibility * rig.color[0].xyz * surface_brdf(n, v, sun, flake_roughness, flake_color);
        color += flake_strength * occlusion * (flake_env * layer_fresnel(flake_color, nv, flake_roughness)
            * intensity + flake_sun * variation);
    }
    if coat > 0.0 {
        let coat_roughness = select(coat_width, 0.18, kind == 7u || kind == 8u);
        let coat_f = layer_fresnel(vec3<f32>(0.04), nv, coat_roughness) * coat;
        let coat_env = layer_environment(coat_ray, coat_roughness, sharp);
        let coat_sun = visibility * rig.color[0].xyz * surface_brdf(n, v, sun, coat_roughness, vec3<f32>(0.04));
        color = color * (vec3<f32>(1.0) - coat_f) + occlusion
            * (coat_env * coat_f * intensity + coat_sun * coat);
    }
    return vec4<f32>(paint_encoded(color), opacity);
}
