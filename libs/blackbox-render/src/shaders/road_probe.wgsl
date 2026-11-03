// A shared car capture is not an infinitely distant panorama. Reject nearby captured surfaces
// when their displacement from a road receiver makes that approximation unsafe. RT/SSR hits
// remain independent of this fallback; no extra capture or ray is required.
@group(1) @binding(13) var road_probe_depth: texture_depth_2d_array;
@group(1) @binding(14) var<uniform> road_probe_origin: vec4<f32>;
struct RoadSky { faces: array<vec4<f32>, 6>, }
@group(1) @binding(15) var<uniform> road_sky: RoadSky;

fn road_sky_at(ray: vec3<f32>) -> vec3<f32> {
    // A broad background estimate needs six colours, not another sampled-texture binding.
    let weight = ray * ray / max(dot(ray, ray), 1e-6);
    return road_sky.faces[select(1, 0, ray.x >= 0.0)].rgb * weight.x
        + road_sky.faces[select(3, 2, ray.y >= 0.0)].rgb * weight.y
        + road_sky.faces[select(5, 4, ray.z >= 0.0)].rgb * weight.z;
}

fn road_probe_radius(direction: vec3<f32>) -> f32 {
    let a = abs(direction);
    var face = select(1, 0, direction.x >= 0.0);
    var coordinate = vec2(select(direction.z, -direction.z, direction.x >= 0.0), -direction.y);
    var major = a.x;
    if a.z >= a.x && a.z >= a.y {
        major = a.z;
        face = select(5, 4, direction.z >= 0.0);
        coordinate = vec2(select(-direction.x, direction.x, direction.z >= 0.0), -direction.y);
    }
    if a.y > a.z && a.y >= a.x {
        major = a.y;
        face = select(3, 2, direction.y >= 0.0);
        coordinate = vec2(direction.x, select(-direction.z, direction.z, direction.y >= 0.0));
    }
    let uv = coordinate / max(major, 1e-6) * vec2(0.5, -0.5) + 0.5;
    let size = vec2<i32>(textureDimensions(road_probe_depth));
    let pixel = clamp(vec2<i32>(uv * vec2<f32>(size)), vec2(0), size - 1);
    let depth = textureLoad(road_probe_depth, pixel, face, 0);
    if depth <= 0.0 { return 1e6; }
    return 0.25 / (depth * max(major, 1e-6));
}

fn road_probe_confidence(world: vec3<f32>, ray: vec3<f32>) -> f32 {
    // Zero denotes a static environment. Negative denotes a valid capture with car correction disabled.
    if abs(road_probe_origin.w) < 0.5 { return 1.0; }
    let radius = road_probe_radius(ray);
    let displacement = length(world - road_probe_origin.xyz);
    // Fade by angular parallax, not a fixed prop size/name or distance from the camera.
    return 1.0 - smoothstep(0.1, 0.4, displacement / max(radius, 0.01));
}

fn road_probe_at(world: vec3<f32>, ray: vec3<f32>, roughness: f32) -> vec3<f32> {
    let confidence = road_probe_confidence(world, ray);
    if confidence <= 0.0 { return road_sky_at(ray); }
    let lod = roughness * f32(textureNumLevels(road_filtered_environment) - 1u);
    var colour = textureSampleLevel(road_environment, road_sampler, ray, 0.0).rgb;
    let broad_weight = smoothstep(0.08, 0.32, roughness);
    if broad_weight > 0.0 {
        colour = mix(colour, textureSampleLevel(road_filtered_environment, road_sampler, ray, lod).rgb, broad_weight);
    }
    if confidence >= 1.0 { return colour; }
    return mix(road_sky_at(ray), colour, confidence);
}
