// Original spatial tent PCF / directional PCSS, following NVIDIA's blocker-distance concept.
// No frame seed, jitter, history or reprojection. SHADOW_GROUP selects the material layout.
struct ShadowData {
    matrices: array<mat4x4<f32>, 4>, splits: vec4<f32>, options: vec4<f32>,
    camera: vec4<f32>, forward: vec4<f32>, sun: vec4<f32>,
    texels: vec4<f32>, depth_spans: vec4<f32>, softness: vec4<f32>, contact: vec4<f32>,
    ambient: vec4<f32>, environment: vec4<f32>, world: vec4<f32>, debug: vec4<f32>,
    projected: vec4<f32>,
};
@group(SHADOW_GROUP) @binding(1) var<uniform> shadow_data: ShadowData;
@group(SHADOW_GROUP) @binding(2) var shadow_map: texture_depth_2d_array;
@group(SHADOW_GROUP) @binding(3) var shadow_sampler: sampler_comparison;
@group(SHADOW_GROUP) @binding(4) var contact_map: texture_2d<f32>;
@group(SHADOW_GROUP) @binding(5) var ambient_map: texture_2d<f32>;

// Evaluate derivatives before cascade selection. Authored smooth normals are for lighting;
// the shadow comparison must follow the plane that was actually rasterised.
fn receiver_normal(world: vec3<f32>, authored: vec3<f32>) -> vec3<f32> {
    let plane = cross(dpdx(world), dpdy(world));
    let magnitude = dot(plane, plane);
    let fallback = authored / max(length(authored), 0.0001);
    if magnitude < 1e-14 { return fallback; }
    let n = plane * inverseSqrt(magnitude);
    return select(n, -n, dot(n, fallback) < 0.0);
}

fn ambient_visibility(pixel: vec2<f32>) -> f32 {
    if shadow_data.ambient.z < 0.5 { return 1.0; }
    let p = clamp(vec2<i32>(pixel), vec2<i32>(0), vec2<i32>(shadow_data.ambient.xy) - vec2<i32>(1));
    return textureLoad(ambient_map, p, 0).r;
}

const BLOCKERS = array<vec2<f32>, 16>(
    vec2<f32>(0.0, 0.0), vec2<f32>(0.35, 0.0), vec2<f32>(-0.175, 0.303), vec2<f32>(-0.175, -0.303),
    vec2<f32>(0.46, 0.46), vec2<f32>(-0.46, 0.46), vec2<f32>(-0.46, -0.46), vec2<f32>(0.46, -0.46),
    vec2<f32>(0.924, 0.383), vec2<f32>(0.383, 0.924), vec2<f32>(-0.383, 0.924), vec2<f32>(-0.924, 0.383),
    vec2<f32>(-0.924, -0.383), vec2<f32>(-0.383, -0.924), vec2<f32>(0.383, -0.924), vec2<f32>(0.924, -0.383)
);

fn cascade_visibility(world: vec3<f32>, normal: vec3<f32>, cascade: u32) -> f32 {
    let plane = normal / max(length(normal), 0.0001);
    let n = select(plane, -plane, dot(plane, shadow_data.sun.xyz) < 0.0);
    let size = shadow_data.options.y;
    let texel_world = shadow_data.texels[cascade];
    let span = shadow_data.depth_spans[cascade];
    let biased = world + n * texel_world * 0.3 * (1.0 - abs(dot(n, shadow_data.sun.xyz)));
    let matrix = shadow_data.matrices[cascade];
    let p = matrix * vec4<f32>(biased, 1.0);
    let uv = vec2<f32>(p.x * 0.5 + 0.5, -p.y * 0.5 + 0.5);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || p.z < 0.0 || p.z > 1.0 { return 1.0; }
    let texel = 1.0 / size;
    let axis_u = normalize(vec3<f32>(matrix[0].x, matrix[1].x, matrix[2].x));
    let axis_v = normalize(vec3<f32>(matrix[0].y, matrix[1].y, matrix[2].y));
    let facing = dot(n, shadow_data.sun.xyz);
    // Smooth regularisation at grazing angles avoids switching all slope correction off.
    let inverse_facing = facing / max(facing * facing, 0.0004);
    let gradient = vec2<f32>(dot(n, axis_u), -dot(n, axis_v)) * inverse_facing * (texel_world * size / span);
    // Hardware PCF compares four texels with one reference. Cover its half-texel footprint.
    let bias = max(0.000002, texel_world * 0.12 / span) + dot(abs(gradient), vec2<f32>(texel * 0.5));
    let reference = p.z - bias;
    if shadow_data.options.z <= 0.0 {
        return textureSampleCompareLevel(shadow_map, shadow_sampler, uv, i32(cascade), reference);
    }
    var radius = shadow_data.options.z;
    if shadow_data.softness.y > 0.5 && shadow_data.softness.x > 0.0 {
        let search = clamp(shadow_data.softness.x * p.z * span / texel_world, 2.0, 12.0);
        var separation = 0.0;
        var count = 0.0;
        for (var i = 0; i < 16; i++) {
            let sample_uv = clamp(uv + BLOCKERS[i] * search * texel, vec2<f32>(texel * 0.5), vec2<f32>(1.0 - texel * 0.5));
            let sample_pixel = vec2<i32>(sample_uv * size);
            let sample_center = (vec2<f32>(sample_pixel) + 0.5) * texel;
            let d = textureLoad(shadow_map, sample_pixel, i32(cascade), 0);
            let gap = reference + dot(gradient, sample_center - uv) - d;
            if d < 1.0 && gap > bias { separation += gap * span; count += 1.0; }
        }
        if count > 0.0 {
            radius = max(radius, clamp(separation / count * shadow_data.softness.x / texel_world, 0.0, 8.0));
        }
    }
    var visibility = 0.0;
    var weights = 0.0;
    for (var y = -2; y <= 2; y++) {
        for (var x = -2; x <= 2; x++) {
            let weight = f32((3 - abs(x)) * (3 - abs(y)));
            let offset = vec2<f32>(f32(x), f32(y)) * radius * texel * 0.5;
            let sample_uv = clamp(uv + offset, vec2<f32>(0.5 * texel), vec2<f32>(1.0 - 0.5 * texel));
            visibility += weight * textureSampleCompareLevel(shadow_map, shadow_sampler, sample_uv, i32(cascade), reference + dot(gradient, sample_uv - uv));
            weights += weight;
        }
    }
    return visibility / weights;
}

fn sun_visibility(world: vec3<f32>, normal: vec3<f32>) -> f32 {
    if shadow_data.options.x < 0.5 { return 1.0; }
    if shadow_data.projected.x > 0.5 { return projected_visibility(world); }
    let depth = dot(world - shadow_data.camera.xyz, shadow_data.forward.xyz);
    let count = u32(shadow_data.camera.w);
    let last = shadow_data.splits[count - 1u];
    if depth <= 0.0 || depth >= last { return 1.0; }
    var cascade = count - 1u;
    for (var i = 0u; i < count; i++) {
        if depth < shadow_data.splits[i] { cascade = i; break; }
    }
    var visibility = cascade_visibility(world, normal, cascade);
    let end = shadow_data.splits[cascade];
    let transition = smoothstep(end * 0.8, end, depth);
    if cascade + 1u < count {
        visibility = mix(visibility, cascade_visibility(world, normal, cascade + 1u), transition);
    }
    if cascade + 1u == count { visibility = mix(visibility, 1.0, transition); }
    return visibility;
}

// Single fitted perspective map. Half-texel projection follows the supplied map size.
// Vendor classification and original filter state remain an explicit parity boundary.
fn projected_samples(world: vec3<f32>) -> vec2<f32> {
    let p = shadow_data.matrices[0] * vec4<f32>(world,1.0);
    if p.w <= 0.0 { return vec2<f32>(1.0,0.0); }
    let uv = vec2<f32>(p.x,-p.y)/p.w*0.5 + vec2<f32>(shadow_data.projected.w);
    let depth = p.z/p.w;
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || depth<0.0 || depth>1.0 { return vec2<f32>(1.0,0.0); }
    let size = shadow_data.options.y;
    var visibility = 0.0;
    for (var y=0; y<2; y++) {
        for (var x=0; x<2; x++) {
            let offset = (vec2<f32>(f32(x),f32(y))-vec2<f32>(0.5))/size;
            visibility += textureSampleCompareLevel(shadow_map,shadow_sampler,uv+offset,0,depth);
        }
    }
    return vec2<f32>(visibility*0.25,uv.y);
}

fn projected_visibility(world: vec3<f32>) -> f32 {
    let samples = projected_samples(world);
    return clamp(samples.x+clamp((0.2-samples.y)*5.0,0.0,1.0)+shadow_data.projected.y,0.0,1.0);
}

fn car_direct_visibility(world: vec3<f32>, normal: vec3<f32>, pixel: vec2<f32>) -> f32 {
    if shadow_data.options.x>0.5 && shadow_data.projected.x>0.5 {
        return mix(0.2+shadow_data.projected.y,1.0,projected_samples(world).x);
    }
    return direct_visibility(world,normal,pixel);
}

fn direct_visibility(world: vec3<f32>, normal: vec3<f32>, pixel: vec2<f32>) -> f32 {
    var visibility = sun_visibility(world, normal);
    if shadow_data.contact.z > 0.5 {
        let p = clamp(vec2<i32>(pixel), vec2<i32>(0), vec2<i32>(shadow_data.contact.xy) - vec2<i32>(1));
        visibility = min(visibility, textureLoad(contact_map, p, 0).r);
    }
    return visibility;
}

fn lighting_debug_colour(world: vec3<f32>, normal: vec3<f32>, pixel: vec2<f32>) -> vec3<f32> {
    if shadow_data.debug.x < 1.5 { return vec3<f32>(sun_visibility(world, normal)); }
    if shadow_data.debug.x < 2.5 { return vec3<f32>(ambient_visibility(pixel)); }
    if shadow_data.options.x < 0.5 { return vec3<f32>(0.15); }
    let depth = dot(world - shadow_data.camera.xyz, shadow_data.forward.xyz);
    let colours = array<vec3<f32>, 4>(vec3<f32>(0.9, 0.25, 0.2), vec3<f32>(0.2, 0.8, 0.3),
        vec3<f32>(0.2, 0.4, 0.95), vec3<f32>(0.85, 0.65, 0.2));
    let count = u32(shadow_data.camera.w);
    if depth <= 0.0 || depth >= shadow_data.splits[count - 1u] { return vec3<f32>(0.15); }
    var cascade = count - 1u;
    for (var i = 0u; i < count; i++) {
        if depth < shadow_data.splits[i] { cascade = i; break; }
    }
    var colour = colours[cascade];
    if cascade + 1u < count {
        colour = mix(colour, colours[cascade + 1u], smoothstep(shadow_data.splits[cascade] * 0.8,
            shadow_data.splits[cascade], depth));
    }
    return colour;
}
