// Copyright (C) 2013 Jorge Jimenez (jorge@iryoku.com)
// Copyright (C) 2013 Jose I. Echevarria (joseignacioechevarria@gmail.com)
// Copyright (C) 2013 Belen Masia (bmasia@unizar.es)
// Copyright (C) 2013 Fernando Navarro (fernandn@microsoft.com)
// Copyright (C) 2013 Diego Gutierrez (diegog@unizar.es)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software. As clarification, there is no
// requirement that the copyright notice and permission be included in binary
// distributions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
// FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
// IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// Search and area helpers of the blending weight pass: diagonal search (SMAASearchDiag1/2, SMAACalculateDiagWeights),
// orthogonal search (SMAASearchXLeft/Right, SMAASearchYUp/Down, SMAASearchLength), the area lookups (SMAAArea,
// SMAAAreaDiag) and the corner rounding (SMAADetectHorizontal/VerticalCornerPattern). The edge and search textures
// are passed in as arguments, as the reference passes them. Included in the blending weight module only.

// SMAADecodeDiagBilinearAccess: unpacks a bilinear fetch of two edges, then rounds.
fn decode_bilinear(v: f32) -> f32 {
    return v * abs(5.0 * v - 5.0 * 0.75);
}

fn decode_diag_pair(e: vec2<f32>) -> vec2<f32> {
    return round(vec2<f32>(decode_bilinear(e.x), e.y));
}

fn decode_diag_quad(e: vec4<f32>) -> vec4<f32> {
    return round(vec4<f32>(decode_bilinear(e.x), e.y, decode_bilinear(e.z), e.w));
}

// `dist` is the reference's returned coord.zw (the walked distance and whether the end was reached);
// `edge` is its `out` parameter, the last edge pair fetched.
struct DiagSearch {
    dist: vec2<f32>,
    edge: vec2<f32>,
}

fn search_diag1(edges_tex: texture_2d<f32>, texcoord: vec2<f32>, dir: vec2<f32>) -> DiagSearch {
    var coord = vec4<f32>(texcoord, -1.0, 1.0);
    let t = vec3<f32>(rt_metrics.xy, 1.0);
    var e = vec2<f32>(0.0);
    while coord.z < SMAA_MAX_SEARCH_STEPS_DIAG - 1.0 && coord.w > 0.9 {
        coord = vec4<f32>(coord.xyz + t * vec3<f32>(dir, 1.0), coord.w);
        e = textureSampleLevel(edges_tex, linear_sampler, coord.xy, 0.0).rg;
        coord.w = dot(e, vec2<f32>(0.5));
    }
    return DiagSearch(coord.zw, e);
}

fn search_diag2(edges_tex: texture_2d<f32>, texcoord: vec2<f32>, dir: vec2<f32>) -> DiagSearch {
    var coord = vec4<f32>(texcoord, -1.0, 1.0);
    coord.x += 0.25 * rt_metrics.x;
    let t = vec3<f32>(rt_metrics.xy, 1.0);
    var e = vec2<f32>(0.0);
    while coord.z < SMAA_MAX_SEARCH_STEPS_DIAG - 1.0 && coord.w > 0.9 {
        coord = vec4<f32>(coord.xyz + t * vec3<f32>(dir, 1.0), coord.w);
        // Both edges of the pixel in one bilinear fetch.
        e = decode_diag_pair(textureSampleLevel(edges_tex, linear_sampler, coord.xy, 0.0).rg);
        coord.w = dot(e, vec2<f32>(0.5));
    }
    return DiagSearch(coord.zw, e);
}

// SMAAAreaDiag: the diagonal area table is in the second half of the texture.
fn area_diag(area_tex: texture_2d<f32>, dist: vec2<f32>, e: vec2<f32>, subtex: f32) -> vec2<f32> {
    var tc = SMAA_AREATEX_MAX_DISTANCE_DIAG * e + dist;
    tc = SMAA_AREATEX_PIXEL_SIZE * tc + 0.5 * SMAA_AREATEX_PIXEL_SIZE;
    tc.x += 0.5;
    tc.y += SMAA_AREATEX_SUBTEX_SIZE * subtex;
    return textureSampleLevel(area_tex, linear_sampler, tc, 0.0).rg;
}

// SMAACalculateDiagWeights, with the two diagonal searches and the crossing-edge fetches.
fn calculate_diag_weights(
    edges_tex: texture_2d<f32>,
    area_tex: texture_2d<f32>,
    texcoord: vec2<f32>,
    e: vec2<f32>,
) -> vec2<f32> {
    var weights = vec2<f32>(0.0);
    var d = vec4<f32>(0.0);

    // Search for the line ends.
    if e.x > 0.0 {
        let s = search_diag1(edges_tex, texcoord, vec2<f32>(-1.0, 1.0));
        d.x = s.dist.x + select(0.0, 1.0, s.edge.y > 0.9);
        d.z = s.dist.y;
    }
    let s_down = search_diag1(edges_tex, texcoord, vec2<f32>(1.0, -1.0));
    d.y = s_down.dist.x;
    d.w = s_down.dist.y;

    if d.x + d.y > 2.0 {
        // Fetch the crossing edges.
        let coords = vec4<f32>(-d.x + 0.25, d.x, d.y, -d.y - 0.25) * rt_metrics.xyxy + texcoord.xyxy;
        let left = textureSampleLevel(edges_tex, linear_sampler, coords.xy, 0.0, vec2<i32>(-1, 0)).rg;
        let right = textureSampleLevel(edges_tex, linear_sampler, coords.zw, 0.0, vec2<i32>(1, 0)).rg;
        let v = decode_diag_quad(vec4<f32>(left, right));
        let c = vec4<f32>(v.y, v.x, v.w, v.z);

        // Merge the crossing edges at each side into one value.
        var cc = 2.0 * vec2<f32>(c.x, c.z) + vec2<f32>(c.y, c.w);
        // Drop the crossing edge where the line end was not found.
        cc = select(cc, vec2<f32>(0.0), step(vec2<f32>(0.9), d.zw) != vec2<f32>(0.0));
        weights += area_diag(area_tex, d.xy, cc, 0.0);
    }

    // Second pair of searches, towards the top-left and bottom-right.
    let s_up = search_diag2(edges_tex, texcoord, vec2<f32>(-1.0, -1.0));
    d.x = s_up.dist.x;
    d.z = s_up.dist.y;
    d.y = 0.0;
    d.w = 0.0;
    if textureSampleLevel(edges_tex, linear_sampler, texcoord, 0.0, vec2<i32>(1, 0)).r > 0.0 {
        let s_bottom = search_diag2(edges_tex, texcoord, vec2<f32>(1.0, 1.0));
        d.y = s_bottom.dist.x + select(0.0, 1.0, s_bottom.edge.y > 0.9);
        d.w = s_bottom.dist.y;
    }

    if d.x + d.y > 2.0 {
        let coords = vec4<f32>(-d.x, -d.x, d.y, d.y) * rt_metrics.xyxy + texcoord.xyxy;
        let c_a = textureSampleLevel(edges_tex, linear_sampler, coords.xy, 0.0, vec2<i32>(-1, 0)).g;
        let c_b = textureSampleLevel(edges_tex, linear_sampler, coords.xy, 0.0, vec2<i32>(0, -1)).r;
        let c_cd = textureSampleLevel(edges_tex, linear_sampler, coords.zw, 0.0, vec2<i32>(1, 0)).gr;
        var cc = 2.0 * vec2<f32>(c_a, c_cd.x) + vec2<f32>(c_b, c_cd.y);
        cc = select(cc, vec2<f32>(0.0), step(vec2<f32>(0.9), d.zw) != vec2<f32>(0.0));
        weights += area_diag(area_tex, d.xy, cc, 0.0).gr;
    }
    return weights;
}

// SMAASearchLength: the search texture stores the walk length, packed, with the left and right cases side by side.
fn search_length(search_tex: texture_2d<f32>, e: vec2<f32>, side: f32) -> f32 {
    var scale = SMAA_SEARCHTEX_SIZE * vec2<f32>(0.5, -1.0);
    var bias = SMAA_SEARCHTEX_SIZE * vec2<f32>(side, 1.0);
    scale += vec2<f32>(-1.0, 1.0);
    bias += vec2<f32>(0.5, -0.5);
    scale *= SMAA_SEARCHTEX_PACKED_INV;
    bias *= SMAA_SEARCHTEX_PACKED_INV;
    return textureSampleLevel(search_tex, linear_sampler, scale * e + bias, 0.0).r;
}

fn search_x_left(edges_tex: texture_2d<f32>, search_tex: texture_2d<f32>, start: vec2<f32>, limit: f32) -> f32 {
    var tc = start;
    var e = vec2<f32>(0.0, 1.0);
    while tc.x > limit && e.y > 0.8281 && e.x == 0.0 {
        e = textureSampleLevel(edges_tex, linear_sampler, tc, 0.0).rg;
        tc = rt_metrics.xy * vec2<f32>(-2.0, 0.0) + tc;
    }
    let offset = -(255.0 / 127.0) * search_length(search_tex, e, 0.0) + 3.25;
    return rt_metrics.x * offset + tc.x;
}

fn search_x_right(edges_tex: texture_2d<f32>, search_tex: texture_2d<f32>, start: vec2<f32>, limit: f32) -> f32 {
    var tc = start;
    var e = vec2<f32>(0.0, 1.0);
    while tc.x < limit && e.y > 0.8281 && e.x == 0.0 {
        e = textureSampleLevel(edges_tex, linear_sampler, tc, 0.0).rg;
        tc = rt_metrics.xy * vec2<f32>(2.0, 0.0) + tc;
    }
    let offset = -(255.0 / 127.0) * search_length(search_tex, e, 0.5) + 3.25;
    return -rt_metrics.x * offset + tc.x;
}

fn search_y_up(edges_tex: texture_2d<f32>, search_tex: texture_2d<f32>, start: vec2<f32>, limit: f32) -> f32 {
    var tc = start;
    var e = vec2<f32>(1.0, 0.0);
    while tc.y > limit && e.x > 0.8281 && e.y == 0.0 {
        e = textureSampleLevel(edges_tex, linear_sampler, tc, 0.0).rg;
        tc = rt_metrics.xy * vec2<f32>(0.0, -2.0) + tc;
    }
    let offset = -(255.0 / 127.0) * search_length(search_tex, e.yx, 0.0) + 3.25;
    return rt_metrics.y * offset + tc.y;
}

fn search_y_down(edges_tex: texture_2d<f32>, search_tex: texture_2d<f32>, start: vec2<f32>, limit: f32) -> f32 {
    var tc = start;
    var e = vec2<f32>(1.0, 0.0);
    while tc.y < limit && e.x > 0.8281 && e.y == 0.0 {
        e = textureSampleLevel(edges_tex, linear_sampler, tc, 0.0).rg;
        tc = rt_metrics.xy * vec2<f32>(0.0, 2.0) + tc;
    }
    let offset = -(255.0 / 127.0) * search_length(search_tex, e.yx, 0.5) + 3.25;
    return -rt_metrics.y * offset + tc.y;
}

// SMAAArea: the orthogonal area table, indexed by the two edge ends and the walked distances.
fn area_ortho(area_tex: texture_2d<f32>, dist: vec2<f32>, e1: f32, e2: f32, subtex: f32) -> vec2<f32> {
    var tc = SMAA_AREATEX_MAX_DISTANCE * round(4.0 * vec2<f32>(e1, e2)) + dist;
    tc = SMAA_AREATEX_PIXEL_SIZE * tc + 0.5 * SMAA_AREATEX_PIXEL_SIZE;
    tc.y = SMAA_AREATEX_SUBTEX_SIZE * subtex + tc.y;
    return textureSampleLevel(area_tex, linear_sampler, tc, 0.0).rg;
}

// SMAADetectHorizontalCornerPattern: `tc` is (left x, row, right x, row).
fn detect_horizontal_corner(
    edges_tex: texture_2d<f32>,
    weights: vec2<f32>,
    tc: vec4<f32>,
    d: vec2<f32>,
) -> vec2<f32> {
    let left_right = step(d, d.yx);
    var rounding = (1.0 - SMAA_CORNER_ROUNDING_NORM) * left_right;
    // Less blending for pixels in the middle of a line.
    rounding /= left_right.x + left_right.y;

    var factor = vec2<f32>(1.0);
    factor.x -= rounding.x * textureSampleLevel(edges_tex, linear_sampler, tc.xy, 0.0, vec2<i32>(0, 1)).r;
    factor.x -= rounding.y * textureSampleLevel(edges_tex, linear_sampler, tc.zw, 0.0, vec2<i32>(1, 1)).r;
    factor.y -= rounding.x * textureSampleLevel(edges_tex, linear_sampler, tc.xy, 0.0, vec2<i32>(0, -2)).r;
    factor.y -= rounding.y * textureSampleLevel(edges_tex, linear_sampler, tc.zw, 0.0, vec2<i32>(1, -2)).r;
    return weights * saturate(factor);
}

// SMAADetectVerticalCornerPattern: `tc` is (column x, top row, column x, bottom row).
fn detect_vertical_corner(
    edges_tex: texture_2d<f32>,
    weights: vec2<f32>,
    tc: vec4<f32>,
    d: vec2<f32>,
) -> vec2<f32> {
    let left_right = step(d, d.yx);
    var rounding = (1.0 - SMAA_CORNER_ROUNDING_NORM) * left_right;
    rounding /= left_right.x + left_right.y;

    var factor = vec2<f32>(1.0);
    factor.x -= rounding.x * textureSampleLevel(edges_tex, linear_sampler, tc.xy, 0.0, vec2<i32>(1, 0)).g;
    factor.x -= rounding.y * textureSampleLevel(edges_tex, linear_sampler, tc.zw, 0.0, vec2<i32>(1, 1)).g;
    factor.y -= rounding.x * textureSampleLevel(edges_tex, linear_sampler, tc.xy, 0.0, vec2<i32>(-2, 0)).g;
    factor.y -= rounding.y * textureSampleLevel(edges_tex, linear_sampler, tc.zw, 0.0, vec2<i32>(-2, 1)).g;
    return weights * saturate(factor);
}
