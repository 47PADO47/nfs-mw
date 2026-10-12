// Shared projection, depth and geometry helpers for the experimental AO passes.
struct AoParams {
    sizes: vec4<f32>, // scene width/height, AO width/height
    projection: vec4<f32>, // inverse projection X/Y scale, reverse-Z near, unused
    effect: vec4<f32>, // radius, intensity, distance fade start/end
    sampling: vec4<u32>, // resolution divisor, slices, steps per side, comparison
    fog: vec4<f32>, // retained uniform layout
    view: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> params: AoParams;
@group(0) @binding(1) var scene_depth: texture_depth_2d;
@group(0) @binding(4) var world_normals: texture_2d<f32>;

fn inside(pixel: vec2<i32>) -> bool {
    return all(pixel >= vec2<i32>(0)) && all(pixel < vec2<i32>(params.sizes.xy));
}
fn depth_at(pixel: vec2<i32>) -> f32 {
    if !inside(pixel) { return 0.0; }
    return textureLoad(scene_depth, pixel, 0);
}
fn view_at(pixel: vec2<f32>, z: f32) -> vec3<f32> {
    let uv = (pixel + 0.5) / params.sizes.xy;
    return vec3<f32>((uv.x * 2.0 - 1.0) * params.projection.x,
                    (1.0 - uv.y * 2.0) * params.projection.y, -1.0) * z;
}
fn position_at(pixel: vec2<i32>) -> vec3<f32> {
    let depth = depth_at(pixel);
    if depth <= 0.0 { return vec3<f32>(0.0); }
    return view_at(vec2<f32>(pixel), params.projection.z / depth);
}
fn safe_normal(v: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    let magnitude = dot(v, v);
    if magnitude < 1e-12 { return fallback; }
    return v * inverseSqrt(magnitude);
}
// Select the derivative with the smallest second-order depth error, so a nearby silhouette
// cannot turn the receiver's normal into a bridge between unrelated surfaces.
fn derivative(pixel: vec2<i32>, axis: vec2<i32>, center: vec3<f32>) -> vec3<f32> {
    let left = position_at(pixel - axis);
    let right = position_at(pixel + axis);
    let left2 = position_at(pixel - axis * 2);
    let right2 = position_at(pixel + axis * 2);
    let le = abs(2.0 * left.z - left2.z - center.z);
    let re = abs(2.0 * right.z - right2.z - center.z);
    if right.z == 0.0 || (left.z != 0.0 && le < re) { return center - left; }
    return right - center;
}
fn normal_at(pixel: vec2<i32>, center: vec3<f32>) -> vec3<f32> {
    let fallback = safe_normal(-center, vec3<f32>(0.0, 0.0, 1.0));
    let encoded = textureLoad(world_normals, pixel, 0);
    if encoded.a > 0.5 {
        let n = safe_normal((params.view * vec4<f32>(encoded.xyz * 2.0 - 1.0, 0.0)).xyz, fallback);
        return select(n, -n, dot(n, fallback) < 0.0);
    }
    var normal = safe_normal(cross(derivative(pixel, vec2<i32>(1, 0), center),
                                  derivative(pixel, vec2<i32>(0, 1), center)), fallback);
    if dot(normal, fallback) < 0.0 { normal = -normal; }
    return normal;
}
fn sign_nonzero(v: vec2<f32>) -> vec2<f32> {
    return select(vec2<f32>(-1.0), vec2<f32>(1.0), v >= vec2<f32>(0.0));
}
fn encode_normal(n: vec3<f32>) -> vec2<f32> {
    let oct = n / (abs(n.x) + abs(n.y) + abs(n.z));
    if oct.z < 0.0 { return (1.0 - abs(oct.yx)) * sign_nonzero(oct.xy); }
    return oct.xy;
}
fn decode_normal(oct: vec2<f32>) -> vec3<f32> {
    var n = vec3<f32>(oct, 1.0 - abs(oct.x) - abs(oct.y));
    if n.z < 0.0 { n = vec3<f32>((1.0 - abs(n.yx)) * sign_nonzero(n.xy), n.z); }
    return normalize(n);
}
fn ao_pixel(pixel: vec2<i32>) -> vec2<i32> {
    return min(pixel * i32(params.sampling.x) + vec2<i32>(i32(params.sampling.x / 2u)),
               vec2<i32>(params.sizes.xy) - 1);
}
// Plane distance tolerates perspective slopes but rejects foreground/background depth jumps.
// Normal agreement further prevents filtering across corners.
fn surface_weight(center: vec3<f32>, normal: vec3<f32>, pixel: vec2<i32>, data: vec4<f32>) -> f32 {
    if data.y <= 0.0 { return 0.0; }
    return geometry_weight(center, normal, view_at(vec2<f32>(ao_pixel(pixel)), data.y), decode_normal(data.zw));
}
fn geometry_weight(center: vec3<f32>, normal: vec3<f32>, neighbor: vec3<f32>, neighbor_normal: vec3<f32>) -> f32 {
    let delta = neighbor - center;
    let tolerance = 0.04 + abs(center.z) * 0.0015;
    let plane = abs(dot(delta, normal)) / tolerance;
    let agreement = max(dot(normal, neighbor_normal), 0.0);
    // The integer power needs three multiplies, without logarithm/exponent evaluation.
    let squared = agreement * agreement;
    let fourth = squared * squared;
    return exp(-plane * plane) * (fourth * fourth);
}
