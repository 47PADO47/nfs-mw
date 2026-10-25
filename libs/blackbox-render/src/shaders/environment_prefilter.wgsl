// Independent implementation of GGX importance sampling; theory: docs/specs/custom-paint-response.md.
@group(0) @binding(0) var source: texture_cube<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var output: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(3) var<uniform> params: vec4<f32>;
@group(0) @binding(4) var linear_source: texture_cube<f32>;

fn radiance(direction: vec3<f32>, level: f32, linear: bool) -> vec3<f32> {
    if linear { return textureSampleLevel(linear_source, source_sampler, direction, level).rgb; }
    return textureSampleLevel(source, source_sampler, direction, level).rgb;
}

fn face_direction(face: u32, uv: vec2<f32>) -> vec3<f32> {
    let p = uv * 2.0 - 1.0;
    switch face {
        case 0u: { return normalize(vec3<f32>(1.0, -p.y, -p.x)); }
        case 1u: { return normalize(vec3<f32>(-1.0, -p.y, p.x)); }
        case 2u: { return normalize(vec3<f32>(p.x, 1.0, p.y)); }
        case 3u: { return normalize(vec3<f32>(p.x, -1.0, -p.y)); }
        case 4u: { return normalize(vec3<f32>(p.x, -p.y, 1.0)); }
        default: { return normalize(vec3<f32>(-p.x, -p.y, -1.0)); }
    }
}

@compute @workgroup_size(8, 8, 1)
fn prefilter_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = u32(params.y);
    if id.x >= size || id.y >= size { return; }
    let n = face_direction(id.z % 6u, (vec2<f32>(id.xy) + 0.5) / params.y);
    let linear = id.z >= 6u;
    if params.x < 0.001 {
        let color = radiance(n, log2(f32(textureDimensions(source).x) / params.y), linear);
        textureStore(output, vec2<i32>(id.xy), i32(id.z), vec4<f32>(color, 1.0));
        return;
    }
    var up = vec3<f32>(0.0, 0.0, 1.0);
    if abs(n.z) > 0.99 { up = vec3<f32>(1.0, 0.0, 0.0); }
    let tangent = normalize(cross(up, n));
    let bitangent = cross(n, tangent);
    let a = params.x * params.x;
    let a2 = a * a;
    var color = vec3<f32>(0.0);
    var weight = 0.0;
    for (var i = 0u; i < 64u; i++) {
        let u = (f32(i) + 0.5) / 64.0;
        let phi = 6.28318530718 * f32(reverseBits(i)) * 2.32830643654e-10;
        let c = sqrt((1.0 - u) / max(1.0 + (a2 - 1.0) * u, 1e-6));
        let s = sqrt(max(1.0 - c * c, 0.0));
        let h = tangent * (cos(phi) * s) + bitangent * (sin(phi) * s) + n * c;
        let l = 2.0 * dot(n, h) * h - n;
        let nl = max(dot(n, l), 0.0);
        if nl <= 0.0 { continue; }
        // Integrate the solid angle represented by each sample, rather than sampling sharp
        // high-frequency texels sparsely across a broad lobe. V=N makes the GGX PDF D/4.
        let denominator = max(c * c * (a2 - 1.0) + 1.0, 1e-6);
        let pdf = max(a2 / (12.56637061436 * denominator * denominator), 1e-6);
        let sample_angle = 1.0 / (64.0 * pdf);
        let source_size = f32(textureDimensions(source).x);
        let texel_angle = 12.56637061436 / (6.0 * source_size * source_size);
        let lod = max(0.0, 0.5 * log2(sample_angle / texel_angle));
        color += radiance(l, lod, linear) * nl;
        weight += nl;
    }
    textureStore(output, vec2<i32>(id.xy), i32(id.z), vec4<f32>(color / max(weight, 1e-6), 1.0));
}
