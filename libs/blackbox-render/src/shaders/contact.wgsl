// Short spatial rays with actual normals, coplanar rejection and finite thickness.
struct Params { inverse: mat4x4<f32>, view_proj: mat4x4<f32>, sun: vec4<f32>, screen: vec4<f32> };
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var depth: texture_depth_2d;
@group(0) @binding(2) var normals: texture_2d<f32>;
fn unproject(uv: vec2<f32>, d: f32) -> vec3<f32> {
    let p = params.inverse * vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, d, 1.0);
    return p.xyz / p.w;
}
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}
@fragment fn fs_main(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let d = textureLoad(depth, vec2<i32>(pixel.xy), 0);
    if d <= 0.0 { return vec4<f32>(1.0); }
    let uv = pixel.xy / params.screen.xy;
    let encoded = textureLoad(normals, vec2<i32>(pixel.xy), 0);
    if encoded.a < 0.5 { return vec4<f32>(1.0); }
    let n = normalize(encoded.xyz * 2.0 - 1.0);
    let facing = dot(n, params.sun.xyz);
    if facing <= 0.08 { return vec4<f32>(1.0); }
    let world = unproject(uv, d);
    let footprint = params.screen.z / d * params.screen.w;
    let bias = clamp(footprint * 0.5, 0.002, 0.015);
    let origin = world + n * bias;
    var shadow = 0.0;
    for (var i = 1; i <= 24; i++) {
        let travel = params.sun.w * f32(i) / 24.0;
        let point = origin + params.sun.xyz * travel;
        let ray = params.view_proj * vec4<f32>(point, 1.0);
        if ray.w <= 0.0 { break; }
        let ndc = ray.xyz / ray.w;
        let sample_uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
        if any(sample_uv < vec2<f32>(0.0)) || any(sample_uv >= vec2<f32>(1.0)) { break; }
        let actual = textureLoad(depth, vec2<i32>(sample_uv * params.screen.xy), 0);
        if actual <= 0.0 { continue; }
        let sample_center = (floor(sample_uv * params.screen.xy) + vec2<f32>(0.5)) / params.screen.xy;
        let hit = unproject(sample_center, actual);
        // A wall/road sampling itself is not an occluder, including steep camera angles.
        if dot(hit - world, n) <= bias * 2.0 { continue; }
        let delta = params.screen.z / max(ndc.z, 0.000001) - params.screen.z / max(actual, 0.000001);
        let thickness = clamp(footprint * 1.25 + params.sun.w / 24.0, 0.015, 0.06);
        if delta > bias && delta < thickness && distance(hit, point) < thickness * 2.0 {
            let edge = min(min(sample_uv.x, sample_uv.y), min(1.0 - sample_uv.x, 1.0 - sample_uv.y));
            shadow = (1.0 - f32(i) / 28.0) * smoothstep(0.0, 0.025, edge) * smoothstep(0.08, 0.3, facing);
            break;
        }
    }
    return vec4<f32>(vec3<f32>(1.0 - shadow * 0.6), 1.0);
}
