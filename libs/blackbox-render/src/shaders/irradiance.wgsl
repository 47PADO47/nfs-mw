// Original SH projection of cube radiance. Nine diffuse coefficients per hemisphere.
// Solid-angle quadrature and Lambertian SH convolution; no temporal sampling or image history.
@group(0) @binding(0) var radiance: texture_cube<f32>;
@group(0) @binding(1) var radiance_sampler: sampler;
@group(0) @binding(2) var<storage, read_write> coefficients: array<vec4<f32>, 18>;

fn direction(face: u32, u: f32, v: f32) -> vec3<f32> {
    switch face {
        case 0u: { return normalize(vec3<f32>(1.0, -v, -u)); }
        case 1u: { return normalize(vec3<f32>(-1.0, -v, u)); }
        case 2u: { return normalize(vec3<f32>(u, 1.0, v)); }
        case 3u: { return normalize(vec3<f32>(u, -1.0, -v)); }
        case 4u: { return normalize(vec3<f32>(u, -v, 1.0)); }
        default: { return normalize(vec3<f32>(-u, -v, -1.0)); }
    }
}
fn basis(i: u32, n: vec3<f32>) -> f32 {
    switch i {
        case 0u: { return 0.282095; }
        case 1u: { return 0.488603 * n.y; }
        case 2u: { return 0.488603 * n.z; }
        case 3u: { return 0.488603 * n.x; }
        case 4u: { return 1.092548 * n.x * n.y; }
        case 5u: { return 1.092548 * n.y * n.z; }
        case 6u: { return 0.315392 * (3.0 * n.z * n.z - 1.0); }
        case 7u: { return 1.092548 * n.x * n.z; }
        default: { return 0.546274 * (n.x * n.x - n.y * n.y); }
    }
}
@compute @workgroup_size(3, 3)
fn project(@builtin(local_invocation_index) index: u32) {
    var sky = vec3<f32>(0.0);
    var ground = vec3<f32>(0.0);
    var total_weight = 0.0;
    for (var face = 0u; face < 6u; face++) {
        for (var y = 0u; y < 8u; y++) {
            for (var x = 0u; x < 8u; x++) {
                let uv = (vec2<f32>(f32(x), f32(y)) + 0.5) / 8.0 * 2.0 - 1.0;
                let n = direction(face, uv.x, uv.y);
                let weight = 1.0 / pow(1.0 + dot(uv, uv), 1.5);
                let sample = textureSampleLevel(radiance, radiance_sampler, n, 0.0).rgb;
                // Match the existing artistic display-space car lighting. This is not an HDR GI bake.
                let term = sample * (weight * basis(index, n));
                if n.z >= 0.0 { sky += term; }
                if n.z < 0.0 { ground += term; }
                total_weight += weight;
            }
        }
    }
    var convolution = 1.0;
    if index >= 1u && index <= 3u { convolution = 2.0 / 3.0; }
    if index >= 4u { convolution = 0.25; }
    let scale = 12.56637061436 * convolution / total_weight;
    coefficients[index] = vec4<f32>(sky * scale, 0.0);
    coefficients[index + 9u] = vec4<f32>(ground * scale, 0.0);
}
