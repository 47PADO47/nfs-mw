// Separate encoded and linear pyramids preserve the two existing material contracts.
@group(0) @binding(0) var source: texture_cube<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var output: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(3) var<uniform> params: vec4<u32>;
@group(0) @binding(4) var previous: texture_2d_array<f32>;

fn direction(face: u32, uv: vec2<f32>) -> vec3<f32> {
    let p = uv * 2.0 - 1.0;
    switch face {
        case 0u: { return vec3(1.0, -p.y, -p.x); }
        case 1u: { return vec3(-1.0, -p.y, p.x); }
        case 2u: { return vec3(p.x, 1.0, p.y); }
        case 3u: { return vec3(p.x, -1.0, -p.y); }
        case 4u: { return vec3(p.x, -p.y, 1.0); }
        default: { return vec3(-p.x, -p.y, -1.0); }
    }
}

@compute @workgroup_size(8, 8, 1)
fn seed(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= vec2(params.x)) { return; }
    let size = f32(params.x);
    // Cover the source footprint before reducing it. Up to 8x8 source texels per output;
    // this is a bounded antialiasing filter, not point sampling a large capture into a small cube.
    let taps = u32(clamp(ceil(f32(textureDimensions(source).x) / size), 1.0, 8.0));
    var color = vec3(0.0);
    for (var y = 0u; y < taps; y++) {
        for (var x = 0u; x < taps; x++) {
            let uv = (vec2<f32>(id.xy) + (vec2<f32>(f32(x), f32(y)) + 0.5) / f32(taps)) / size;
            var c = textureSampleLevel(source, source_sampler, direction(id.z % 6u, uv), 0.0).rgb;
            if id.z >= 6u {
                c = select(pow((c + 0.055) / 1.055, vec3(2.4)), c / 12.92, c <= vec3(0.04045));
            }
            color += c;
        }
    }
    textureStore(output, vec2<i32>(id.xy), i32(id.z), vec4(color / f32(taps * taps), 1.0));
}

@compute @workgroup_size(8, 8, 1)
fn reduce(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= vec2(params.x)) { return; }
    var color = vec3(0.0);
    for (var y = 0u; y < 2u; y++) {
        for (var x = 0u; x < 2u; x++) {
            let pixel = vec2<i32>(id.xy * 2u + vec2(x, y));
            color += textureLoad(previous, pixel, i32(id.z), 0).rgb;
        }
    }
    textureStore(output, vec2<i32>(id.xy), i32(id.z), vec4(color * 0.25, 1.0));
}
