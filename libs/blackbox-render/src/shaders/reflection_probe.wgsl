// Depth-assisted local probe reprojection. All depth belongs to the current six-face capture.
@group(2) @binding(10) var probe_depth: texture_depth_2d_array;
struct LocalProbe { origin: vec4<f32>, };
@group(2) @binding(11) var<uniform> local_probe: LocalProbe;

fn probe_radius(direction: vec3<f32>) -> f32 {
    let a = abs(direction);
    var face = 0;
    var coordinate = vec2<f32>(-direction.z, -direction.y);
    var major = a.x;
    if a.z >= a.x && a.z >= a.y {
        major = a.z;
        face = select(5, 4, direction.z >= 0.0);
        coordinate = vec2(select(-direction.x, direction.x, direction.z >= 0.0), -direction.y);
    } else if a.y >= a.x {
        major = a.y;
        face = select(3, 2, direction.y >= 0.0);
        coordinate = vec2(direction.x, select(-direction.z, direction.z, direction.y >= 0.0));
    } else {
        face = select(1, 0, direction.x >= 0.0);
        coordinate = vec2(select(direction.z, -direction.z, direction.x >= 0.0), -direction.y);
    }
    var uv = coordinate / max(major, 1e-6) * 0.5 + vec2<f32>(0.5);
    uv.y = 1.0 - uv.y;
    let size = vec2<i32>(textureDimensions(probe_depth));
    let pixel = clamp(vec2<i32>(uv * vec2<f32>(size)), vec2<i32>(0), size - vec2<i32>(1));
    let depth = textureLoad(probe_depth, pixel, face, 0);
    // Reverse infinite-Z, near = 0.25. Sky and distant scenery need no correction.
    if depth <= 0.0025 { return 0.0; }
    return 0.25 / (depth * max(major, 1e-6));
}

fn probe_direction(world: vec3<f32>, ray: vec3<f32>, roughness: f32) -> vec3<f32> {
    if local_probe.origin.w < 0.5 || roughness >= 0.5 { return ray; }
    let offset = world - local_probe.origin.xyz;
    if dot(offset, offset) > 64.0 { return ray; }
    var direction = ray;
    var hit = ray;
    for (var i = 0; i < 3; i++) {
        let radius = probe_radius(direction);
        if radius <= 0.0 { return ray; }
        let along = dot(offset, ray);
        let discriminant = along * along + radius * radius - dot(offset, offset);
        if discriminant <= 0.0 { return ray; }
        let travel = sqrt(discriminant) - along;
        if travel <= 0.05 { return ray; }
        hit = offset + ray * travel;
        direction = normalize(hit);
    }
    let final_radius = probe_radius(direction);
    // Reject disocclusions and non-converged depth edges instead of stretching them.
    let error = abs(final_radius - length(hit));
    let confidence = 1.0 - smoothstep(0.1, max(0.3, final_radius * 0.03), error);
    let roughness_weight = 1.0 - smoothstep(0.18, 0.5, roughness);
    return normalize(mix(ray, direction, confidence * roughness_weight));
}
