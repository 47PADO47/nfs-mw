// Directional PCSS (NVIDIA's blocker-distance concept) with a per-pixel rotated Vogel-disk filter,
// early-outs for fully lit and fully shadowed pixels, and dithered cascade handoffs.
// No frame seed, jitter, history or reprojection. SHADOW_GROUP selects the material layout.
struct ShadowData {
    matrices: array<mat4x4<f32>, 4>, splits: vec4<f32>, options: vec4<f32>,
    camera: vec4<f32>, forward: vec4<f32>, sun: vec4<f32>,
    texels: vec4<f32>, depth_spans: vec4<f32>, softness: vec4<f32>, contact: vec4<f32>,
    ambient: vec4<f32>, environment: vec4<f32>, world: vec4<f32>, debug: vec4<f32>,
    projected: vec4<f32>, resolutions: vec4<f32>,
};
@group(SHADOW_GROUP) @binding(1) var<uniform> shadow_data: ShadowData;
@group(SHADOW_GROUP) @binding(2) var shadow_map: texture_depth_2d_array;
@group(SHADOW_GROUP) @binding(3) var shadow_sampler: sampler_comparison;
@group(SHADOW_GROUP) @binding(4) var contact_map: texture_2d<f32>;
@group(SHADOW_GROUP) @binding(5) var ambient_map: texture_2d<f32>;
@group(SHADOW_GROUP) @binding(6) var detail_shadow_map: texture_depth_2d_array;

fn shadow_size(cascade: u32) -> f32 {
    if shadow_data.softness.w > 0.5 { return shadow_data.resolutions[cascade]; }
    return shadow_data.options.y;
}

fn shadow_compare(uv: vec2<f32>, cascade: u32, reference: f32) -> f32 {
    if shadow_data.softness.w > 0.5 {
        if cascade < 2u {
            return textureSampleCompareLevel(detail_shadow_map,shadow_sampler,uv,i32(cascade),reference);
        }
        return textureSampleCompareLevel(shadow_map,shadow_sampler,uv,i32(cascade - 2u),reference);
    }
    return textureSampleCompareLevel(shadow_map,shadow_sampler,uv,i32(cascade),reference);
}

fn shadow_depth(pixel: vec2<i32>, cascade: u32) -> f32 {
    if shadow_data.softness.w > 0.5 {
        if cascade < 2u { return textureLoad(detail_shadow_map,pixel,i32(cascade),0); }
        return textureLoad(shadow_map,pixel,i32(cascade - 2u),0);
    }
    return textureLoad(shadow_map,pixel,i32(cascade),0);
}

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

// Per-pixel screen position for the rotation and cascade dithering below. Entry points that know
// it set it in direct_visibility; elsewhere it stays zero, which gives a fixed, unrotated pattern.
var<private> shadow_pixel: vec2<f32>;

// Interleaved gradient noise (Jimenez): cheap, stable per pixel, no frame seed or history.
fn interleaved_noise(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2<f32>(0.06711056, 0.00583715))));
}

// Vogel (golden angle) disks, radius sqrt((i + 0.5) / N). Twelve taps for the soft filter,
// twenty for the dense one.
const VOGEL_12 = array<vec2<f32>, 12>(
    vec2<f32>(0.2041, 0.0000), vec2<f32>(-0.2607, 0.2388), vec2<f32>(0.0399, -0.4547),
    vec2<f32>(0.3286, 0.4286), vec2<f32>(-0.6030, -0.1067), vec2<f32>(0.5712, -0.3634),
    vec2<f32>(-0.1911, 0.7107), vec2<f32>(-0.3644, -0.7016), vec2<f32>(0.7906, 0.2887),
    vec2<f32>(-0.8224, 0.3395), vec2<f32>(0.3965, -0.8472), vec2<f32>(0.2930, 0.9341)
);
const VOGEL_20 = array<vec2<f32>, 20>(
    vec2<f32>(0.1581, 0.0000), vec2<f32>(-0.2019, 0.1850), vec2<f32>(0.0309, -0.3522),
    vec2<f32>(0.2545, 0.3320), vec2<f32>(-0.4671, -0.0826), vec2<f32>(0.4425, -0.2815),
    vec2<f32>(-0.1480, 0.5505), vec2<f32>(-0.2822, -0.5434), vec2<f32>(0.6124, 0.2236),
    vec2<f32>(-0.6371, 0.2630), vec2<f32>(0.3071, -0.6563), vec2<f32>(0.2269, 0.7235),
    vec2<f32>(-0.6840, -0.3964), vec2<f32>(0.8024, -0.1764), vec2<f32>(-0.4897, 0.6966),
    vec2<f32>(-0.1131, -0.8730), vec2<f32>(0.6945, 0.5853), vec2<f32>(-0.9346, 0.0386),
    vec2<f32>(0.6817, -0.6784), vec2<f32>(-0.0456, 0.9864)
);

fn disk_tap(i: i32, dense: bool) -> vec2<f32> {
    if dense { return VOGEL_20[i]; }
    return VOGEL_12[i];
}

// Centre-weighted disk of hardware bilinear comparisons, `radius` texels across, rotated per pixel
// so the few taps trade banding for fine noise. Replaces the 5x5 grid and the dense tent.
fn disk_filter(uv: vec2<f32>, cascade: u32, reference: f32, gradient: vec2<f32>, radius: f32, dense: bool) -> f32 {
    let texel = 1.0 / shadow_size(cascade);
    let angle = interleaved_noise(shadow_pixel) * 6.2831853;
    let spin = vec2<f32>(cos(angle), sin(angle));
    var taps = 12;
    if dense { taps = 20; }
    var visibility = 0.0;
    var weights = 0.0;
    for (var i = 0; i < taps; i++) {
        let tap = disk_tap(i, dense);
        let rotated = vec2<f32>(tap.x * spin.x - tap.y * spin.y, tap.x * spin.y + tap.y * spin.x);
        let weight = 1.0 - 0.5 * dot(tap, tap);
        let sample_uv = clamp(uv + rotated * radius * texel, vec2<f32>(0.5 * texel), vec2<f32>(1.0 - 0.5 * texel));
        visibility += weight * shadow_compare(sample_uv,cascade,reference + dot(gradient, sample_uv - uv));
        weights += weight;
    }
    return visibility / weights;
}

// Visibility, accepted blocker fraction, filter radius in units of the eight-texel budget.
fn cascade_sample(world: vec3<f32>, normal: vec3<f32>, cascade: u32) -> vec3<f32> {
    let plane = normal / max(length(normal), 0.0001);
    let n = select(plane, -plane, dot(plane, shadow_data.sun.xyz) < 0.0);
    let size = shadow_size(cascade);
    let texel_world = shadow_data.texels[cascade];
    let span = shadow_data.depth_spans[cascade];
    let biased = world + n * texel_world * 0.3 * (1.0 - abs(dot(n, shadow_data.sun.xyz)));
    let matrix = shadow_data.matrices[cascade];
    let p = matrix * vec4<f32>(biased, 1.0);
    let uv = vec2<f32>(p.x * 0.5 + 0.5, -p.y * 0.5 + 0.5);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || p.z < 0.0 || p.z > 1.0 { return vec3<f32>(1.0,0.0,0.0); }
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
        return vec3<f32>(shadow_compare(uv,cascade,reference),0.0,0.0);
    }
    var radius = shadow_data.options.z;
    var maximum_radius = 8.0;
    if shadow_data.softness.w > 0.5 {
        // One world-space footprint through all overlap bands. The nearest map
        // defines the slider's units; coarser maps still have their own raster limit.
        let scale = shadow_data.texels.x / texel_world;
        radius *= scale;
        maximum_radius *= scale;
    }
    var blocker_fraction = 0.0;
    if shadow_data.softness.y > 0.5 && shadow_data.softness.x > 0.0 {
        // Directional light has no finite depth origin. The old p.z*span footprint
        // changed when unrelated upstream casters expanded the fitted depth slab.
        // Cover the bounded penumbra with a fixed search, within the cascade guard band.
        let search = 12.0;
        var separation = 0.0;
        var count = 0.0;
        for (var i = 0; i < 16; i++) {
            let sample_uv = clamp(uv + BLOCKERS[i] * search * texel, vec2<f32>(texel * 0.5), vec2<f32>(1.0 - texel * 0.5));
            let sample_pixel = vec2<i32>(sample_uv * size);
            let sample_center = (vec2<f32>(sample_pixel) + 0.5) * texel;
            let d = shadow_depth(sample_pixel,cascade);
            let gap = reference + dot(gradient, sample_center - uv) - d;
            if d < 1.0 && gap > bias { separation += gap * span; count += 1.0; }
        }
        // No tap sees a blocker: the receiver is lit, whatever the filter would have averaged.
        if count < 0.5 { return vec3<f32>(1.0,0.0,radius/8.0); }
        radius = max(radius, clamp(separation / count * shadow_data.softness.x / texel_world, 0.0, maximum_radius));
        blocker_fraction = count / 16.0;
        // Every tap is a blocker and the penumbra fits inside the search: fully shadowed.
        if count > 15.5 && radius < 2.5 { return vec3<f32>(0.0,blocker_fraction,radius/8.0); }
    }
    if shadow_data.softness.z > 0.5 && radius > 2.0 {
        let dense = disk_filter(uv,cascade,reference,gradient,radius,true);
        if radius >= 3.0 { return vec3<f32>(dense,blocker_fraction,radius/8.0); }
        // A continuous handoff avoids popping as PCSS estimates cross the narrow-kernel limit.
        let narrow = disk_filter(uv,cascade,reference,gradient,radius,false);
        return vec3<f32>(mix(narrow,dense,smoothstep(2.0,3.0,radius)),blocker_fraction,radius/8.0);
    }
    return vec3<f32>(disk_filter(uv,cascade,reference,gradient,radius,false),blocker_fraction,radius/8.0);
}

fn sun_sample(world: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    if shadow_data.options.x < 0.5 { return vec3<f32>(1.0,0.0,0.0); }
    if shadow_data.projected.x > 0.5 { return vec3<f32>(projected_visibility(world),0.0,0.0); }
    let depth = dot(world - shadow_data.camera.xyz, shadow_data.forward.xyz);
    let count = u32(shadow_data.camera.w);
    let last = shadow_data.splits[count - 1u];
    if depth <= 0.0 || depth >= last { return vec3<f32>(1.0,0.0,0.0); }
    var cascade = count - 1u;
    for (var i = 0u; i < count; i++) {
        if depth < shadow_data.splits[i] { cascade = i; break; }
    }
    let end = shadow_data.splits[cascade];
    let overlap = select(0.8,0.65,shadow_data.softness.w > 0.5 && cascade + 1u < count);
    let transition = smoothstep(end * overlap, end, depth);
    if cascade + 1u < count {
        // Dithered handoff: one cascade per pixel, the next one with probability `transition`.
        // The noise is offset from the filter rotation so the two do not correlate.
        let pick = interleaved_noise(shadow_pixel + vec2<f32>(47.0,13.0));
        if transition > pick { return cascade_sample(world, normal, cascade + 1u); }
        return cascade_sample(world, normal, cascade);
    }
    var sample = cascade_sample(world, normal, cascade);
    sample.x = mix(sample.x, 1.0, transition);
    return sample;
}

fn sun_visibility(world: vec3<f32>, normal: vec3<f32>) -> f32 {
    return sun_sample(world,normal).x;
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
    shadow_pixel = pixel;
    var visibility = sun_visibility(world, normal);
    if shadow_data.contact.z > 0.5 {
        let p = clamp(vec2<i32>(pixel), vec2<i32>(0), vec2<i32>(shadow_data.contact.xy) - vec2<i32>(1));
        visibility = min(visibility, textureLoad(contact_map, p, 0).r);
    }
    return visibility;
}

fn lighting_debug_colour(world: vec3<f32>, normal: vec3<f32>, pixel: vec2<f32>) -> vec3<f32> {
    shadow_pixel = pixel;
    if shadow_data.debug.x > 3.5 {
        let sample = sun_sample(world,normal);
        return vec3<f32>(select(sample.y,sample.z,shadow_data.debug.x>4.5));
    }
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
        let overlap = select(0.8,0.65,shadow_data.softness.w > 0.5);
        colour = mix(colour, colours[cascade + 1u], smoothstep(shadow_data.splits[cascade] * overlap,
            shadow_data.splits[cascade], depth));
    }
    return colour;
}
