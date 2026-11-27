// A translucent sheet of atomized liquid. UV.x crosses the sheet and UV.y is travelled distance in metres.
fn wake_hash(p: vec2<f32>) -> f32 {
    let q = fract(vec3<f32>(p.xyx) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let h = q + dot(q, q.yzx + vec3<f32>(33.33));
    return fract((h.x + h.y) * h.z);
}
fn wake_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let s = f * f * (3.0 - 2.0 * f);
    return mix(mix(wake_hash(i), wake_hash(i + vec2<f32>(1.0, 0.0)), s.x),
        mix(wake_hash(i + vec2<f32>(0.0, 1.0)), wake_hash(i + vec2<f32>(1.0)), s.x), s.y);
}
fn liquid_wake(uv: vec2<f32>, age: f32, seed: f32, world: vec3<f32>, opacity: f32) -> vec4<f32> {
    let pixel_width = fwidth(uv);
    if seed >= 128.0 {
        // Bound the extinction of a long, end-on chord. Several tire parcels
        // overlap along the chase view; road contrast must survive that overlap.
        let r2 = dot(uv, uv);
        let chord = sqrt(max(0.0, 1.0 - r2));
        let flow = vec2<f32>(uv.x * 4.5 + seed, uv.y * 2.4 - age * 2.1);
        let filaments = smoothstep(0.22, 0.76, wake_noise(flow));
        let thickness = opacity * chord * chord * chord;
        let tau = thickness * (0.20 + 0.95 * filaments);
        let alpha = tau / (1.0 + tau) * (1.0 - 0.45 * smoothstep(0.045, 0.21, age));
        let sky = clamp(globals.fog_color.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
        let light = max(dot(normalize(globals.camera_pos.xyz - world), globals.light_dir.xyz), 0.0);
        let color = mix(sky, vec3<f32>(dot(sky, vec3<f32>(0.2126,0.7152,0.0722))), 0.72);
        return vec4<f32>(min(vec3<f32>(1.0), color * (1.02 + 0.32 * light * light) + 0.015), alpha);
    }
    if seed >= 95.0 {
        // Three short filaments cover only part of the projected spray footprint.
        // Use pixel gradients calculated before material selection for subpixel coverage.
        let lane = clamp(floor((uv.x + 1.0) * 1.5), 0.0, 2.0);
        let phase = fract((seed - 96.0) * 13.7 + lane * 0.382);
        let x = uv.x * 3.0 + 2.0 - 2.0 * lane - (phase - 0.5) * 0.55;
        let y = (uv.y - (phase - 0.5) * 0.7) * (1.65 + phase);
        let taper = max(0.0, 1.0 - y * y);
        let radius = (0.15 + 0.14 * phase) * taper;
        let pixel = max(0.025, 3.0 * pixel_width.x);
        let coverage = clamp((radius + pixel * 0.5 - abs(x)) / pixel, 0.0, 1.0)
            * min(1.0, 2.0 * radius / pixel) * taper;
        let sky = clamp(globals.fog_color.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
        return vec4<f32>(sky * (0.85 + phase * 0.24), opacity * coverage);
    }
    // Advect two scales at different rates as the sheet entrains air. Noise stays attached
    // to each parcel and distance along the wheel path as the chase camera moves.
    let flow = uv.y * 0.78 - age * 1.65;
    let curl = wake_noise(vec2<f32>(uv.x * 1.7 + seed + 0.6 * age, flow)) - 0.5;
    let edge = 1.0 - smoothstep(0.10, 1.0, abs(uv.x + curl * 0.36));
    let threads = wake_noise(vec2<f32>(uv.x * 7.0 + curl * (2.4 + 2.0 * age), uv.y * 1.8 + seed - 3.1 * age));
    var density = edge * mix(0.36 + 0.64 * threads, smoothstep(0.32, 0.77, threads), smoothstep(0.04, 0.22, age));
    if seed >= 40.0 {
        let warped = uv + vec2<f32>(0.38 * curl, 0.31 * (threads - 0.5));
        let r2 = dot(warped, warped);
        let core = 1.0 - smoothstep(0.015, 0.86, r2);
        let fringe = smoothstep(0.20, 0.64, threads + 0.26 * curl);
        density = (1.0 - 0.52 * smoothstep(0.08, 0.40, age)) * core * core * mix(0.34 + 0.52 * threads, fringe, smoothstep(0.12, 0.52, age));
    }
    let view = normalize(world - globals.camera_pos.xyz);
    let scatter = pow(max(dot(view, globals.light_dir.xyz), 0.0), 4.0);
    let sky = clamp(globals.fog_color.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    let luminance = dot(sky, vec3<f32>(0.2126, 0.7152, 0.0722));
    let color = mix(sky, vec3<f32>(luminance), 0.45) * (1.22 + 0.65 * scatter) + vec3<f32>(0.035);
    return vec4<f32>(min(color, vec3<f32>(1.0)), opacity * density);
}
