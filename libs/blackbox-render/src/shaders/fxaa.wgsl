// FXAA: fast approximate anti-aliasing in one pass.
//
// An independent implementation of the algorithm Timothy Lottes described for FXAA 3.11 (NVIDIA,
// "FXAA", 2009 white paper; the 3.11 "quality" variant), written from that description. No code of
// the reference header is used. Steps: find edges by local luma contrast; blend sub-pixel detail;
// decide whether the edge runs horizontally or vertically; walk along the edge in both directions
// to its ends; and shift the sample across the edge by how far the pixel is from the nearer end.
//
// params.a = (edge threshold, edge threshold minimum, sub-pixel blend, 0)

const SEARCH_STEPS: i32 = 12;

// How far each step of the edge walk advances, in texels: the walk starts with fine steps and
// speeds up, so long edges are found without many samples.
var<private> STEP_SIZE: array<f32, 12> = array<f32, 12>(1.0, 1.0, 1.0, 1.0, 1.0, 1.5, 2.0, 2.0, 2.0, 2.0, 4.0, 8.0);

// Samples are clamped to the displayable range: with tone mapping off, highlights can exceed 1.0,
// and the resolve pass clamps them anyway.
fn fetch(uv: vec2<f32>) -> vec3<f32> {
    return clamp(textureSampleLevel(src, samp, uv, 0.0).rgb, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn luma_of(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

fn luma_at(p: vec2<i32>) -> f32 {
    let size = vec2<i32>(textureDimensions(src));
    let q = clamp(p, vec2<i32>(0), size - vec2<i32>(1));
    return luma_of(clamp(textureLoad(src, q, 0).rgb, vec3<f32>(0.0), vec3<f32>(1.0)));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let p = vec2<i32>(in.clip.xy);
    let size = vec2<f32>(textureDimensions(src));
    let texel = 1.0 / size;
    let uv = in.uv;
    let centre = textureSampleLevel(src, samp, uv, 0.0);

    let m = luma_at(p);
    let n = luma_at(p + vec2<i32>(0, -1));
    let s = luma_at(p + vec2<i32>(0, 1));
    let w = luma_at(p + vec2<i32>(-1, 0));
    let e = luma_at(p + vec2<i32>(1, 0));
    let luma_max = max(m, max(max(n, s), max(w, e)));
    let luma_min = min(m, min(min(n, s), min(w, e)));
    let range = luma_max - luma_min;
    // Flat or faint areas are left alone.
    if (range < max(params.a.y, luma_max * params.a.x)) {
        return centre;
    }

    let nw = luma_at(p + vec2<i32>(-1, -1));
    let ne = luma_at(p + vec2<i32>(1, -1));
    let sw = luma_at(p + vec2<i32>(-1, 1));
    let se = luma_at(p + vec2<i32>(1, 1));

    // Sub-pixel aliasing: how far the pixel is from the average of its neighbourhood.
    let average = (2.0 * (n + s + e + w) + (nw + ne + sw + se)) / 12.0;
    let contrast = clamp(abs(average - m) / range, 0.0, 1.0);
    let smooth_contrast = contrast * contrast * (3.0 - 2.0 * contrast);
    let sub_pixel = smooth_contrast * smooth_contrast * params.a.z;

    // Is the edge horizontal or vertical? Compare the second derivative along both axes.
    let edge_h = abs(nw - 2.0 * n + ne) + 2.0 * abs(w - 2.0 * m + e) + abs(sw - 2.0 * s + se);
    let edge_v = abs(nw - 2.0 * w + sw) + 2.0 * abs(n - 2.0 * m + s) + abs(ne - 2.0 * e + se);
    let horizontal = edge_h >= edge_v;

    // The two neighbours across the edge; the steeper side is where the edge lies.
    let luma_1 = select(w, n, horizontal);
    let luma_2 = select(e, s, horizontal);
    let gradient_1 = luma_1 - m;
    let gradient_2 = luma_2 - m;
    let first_steeper = abs(gradient_1) >= abs(gradient_2);
    let gradient_scaled = 0.25 * max(abs(gradient_1), abs(gradient_2));
    let local_average = 0.5 * (select(luma_2, luma_1, first_steeper) + m);

    // Move half a texel across the edge so the bilinear fetches average the pixels on both sides.
    var step_length = select(texel.x, texel.y, horizontal);
    if (first_steeper) {
        step_length = -step_length;
    }
    var start = uv;
    if (horizontal) {
        start.y += step_length * 0.5;
    } else {
        start.x += step_length * 0.5;
    }
    let along = select(vec2<f32>(0.0, texel.y), vec2<f32>(texel.x, 0.0), horizontal);

    // Walk along the edge both ways until the luma differs from the edge's average enough.
    var pos_1 = start - along;
    var pos_2 = start + along;
    var end_1 = luma_of(fetch(pos_1)) - local_average;
    var end_2 = luma_of(fetch(pos_2)) - local_average;
    var done_1 = abs(end_1) >= gradient_scaled;
    var done_2 = abs(end_2) >= gradient_scaled;
    for (var i = 0; i < SEARCH_STEPS; i++) {
        if (done_1 && done_2) {
            break;
        }
        if (!done_1) {
            pos_1 -= along * STEP_SIZE[i];
            end_1 = luma_of(fetch(pos_1)) - local_average;
            done_1 = abs(end_1) >= gradient_scaled;
        }
        if (!done_2) {
            pos_2 += along * STEP_SIZE[i];
            end_2 = luma_of(fetch(pos_2)) - local_average;
            done_2 = abs(end_2) >= gradient_scaled;
        }
    }

    let distance_1 = select(uv.y - pos_1.y, uv.x - pos_1.x, horizontal);
    let distance_2 = select(pos_2.y - uv.y, pos_2.x - uv.x, horizontal);
    let nearer_is_1 = distance_1 < distance_2;
    let nearest = min(distance_1, distance_2);
    let thickness = distance_1 + distance_2;
    let pixel_offset = 0.5 - nearest / max(thickness, 1e-6);

    // Only shift when the luma at the nearer end moves the right way (the pixel is on the wrong side).
    let centre_is_darker = m < local_average;
    let end_delta = select(end_2, end_1, nearer_is_1);
    let wrong_side = (end_delta < 0.0) != centre_is_darker;
    let edge_offset = select(0.0, pixel_offset, wrong_side);

    let final_offset = max(edge_offset, sub_pixel);
    var final_uv = uv;
    if (horizontal) {
        final_uv.y += final_offset * step_length;
    } else {
        final_uv.x += final_offset * step_length;
    }
    return vec4<f32>(textureSampleLevel(src, samp, final_uv, 0.0).rgb, centre.a);
}
