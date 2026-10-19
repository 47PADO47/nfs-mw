// Optional current-frame rays on glossy materials; misses retain the authored environment response.
// No frame history, display transfer, UI, or generated frames enter this path.
@group(2) @binding(5) var<uniform> screen_control: vec4<f32>;
@group(2) @binding(6) var screen_colour: texture_2d<f32>;
@group(2) @binding(7) var screen_depth: texture_depth_2d;

fn hybrid_environment(world: vec3<f32>, normal: vec3<f32>, direction: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    if screen_control.x < 0.5 || globals.fog_range.w > 0.5 { return fallback; }
    if distance(world, globals.camera_pos.xyz) > 100.0 { return fallback; }
    let size = vec2<i32>(textureDimensions(screen_depth));
    let origin = world + normal * 0.05;
    let ray = normalize(direction);
    var last_t = 0.0;
    for (var step = 1u; step <= 48u; step++) {
        let fraction = f32(step) / 48.0;
        let t = 0.15 + fraction * fraction * screen_control.z;
        let point = origin + ray * t;
        let clip = globals.view_proj * vec4<f32>(point, 1.0);
        if clip.w <= screen_control.y { return fallback; }
        let uv = clip.xy / clip.w * vec2<f32>(0.5,-0.5) + 0.5;
        if any(uv <= vec2<f32>(0.0)) || any(uv >= vec2<f32>(1.0)) { return fallback; }
        let pixel = vec2<i32>(uv * vec2<f32>(size));
        let depth = textureLoad(screen_depth, pixel, 0);
        if depth <= 0.0 { last_t = t; continue; }
        let scene_z = screen_control.y / depth;
        let delta = clip.w - scene_z;
        if delta >= 0.0 && delta < max(0.15, t * 0.03) {
            var low = last_t; var high = t;
            var hit_uv = uv;
            for (var refine=0u; refine<5u; refine++) {
                let middle = (low + high) * 0.5;
                let c = globals.view_proj * vec4<f32>(origin + ray * middle, 1.0);
                let u = c.xy / c.w * vec2<f32>(0.5,-0.5) + 0.5;
                let p = clamp(vec2<i32>(u * vec2<f32>(size)), vec2<i32>(0), size - 1);
                let d = textureLoad(screen_depth, p, 0);
                if d > 0.0 && c.w >= screen_control.y / d { high = middle; hit_uv = u; continue; }
                low = middle;
            }
            let edge = min(min(hit_uv.x, 1.0-hit_uv.x), min(hit_uv.y, 1.0-hit_uv.y));
            let confidence = smoothstep(0.0, 0.08, edge) * (1.0 - fraction * fraction);
            let colour = textureSampleLevel(screen_colour, env_sampler, hit_uv, 0.0).rgb;
            return mix(fallback, colour, confidence);
        }
        last_t = t;
    }
    return fallback;
}
