// FidelityFX Super Resolution 1 (FSR 1): EASU (edge-adaptive spatial upsampling) and RCAS (robust
// contrast-adaptive sharpening), ported to WGSL from AMD's ffx_fsr1.h (v1.20210629) in
// https://github.com/GPUOpen-Effects/FidelityFX-FSR, with concepts of ffx_a.h (the approximate
// reciprocal and reciprocal square root helpers).
//
// FidelityFX Super Resolution Sample
//
// Copyright (c) 2021 Advanced Micro Devices, Inc. All rights reserved.
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files(the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and / or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions :
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
//
// ffx_a.h, which defines the helpers used here, also carries:
// Copyright (c) 2014 Michal Drobot (for concepts used in "FLOAT APPROXIMATIONS"),
// under the same MIT permission notice as above.
//
// Changes from the original: the 12 EASU taps are fetched with textureLoad at integer positions
// (clamped to the image) instead of gather4, which keeps the shader valid on every backend; the input
// is clamped to 0..1 on load; RCAS uses an exact reciprocal for its final division and keeps the
// limiter divisions away from zero; there is no 16-bit variant and no film grain.
//
// The input must be perceptual (display-referred, gamma-like) colour, already anti-aliased: this is
// what the resolve pass writes to the surface.

struct Params {
    // The size of the image the pass writes, in pixels.
    out_size: vec2<f32>,
    // The size of the image this pass reads, in pixels: the valid content inside `src`, which may be
    // smaller than `textureDimensions(src)` (a caller that draws its low-resolution image into one
    // corner of a fixed-size texture, rather than a texture sized exactly to it, passes its own content
    // size here instead of the texture's).
    in_size: vec2<f32>,
    // RCAS sharpness as a linear lobe scale: exp2(-stops), 1 = maximum sharpness.
    rcas: f32,
    // `vec3`'s own alignment in the uniform address space is 16 bytes, not 12: a lone `f32` keeps this
    // struct free of that padding so its WGSL size matches the Rust `Params`'s plain `repr(C)` layout.
    pad: f32,
};

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var<uniform> params: Params;

// One oversized triangle covers the screen.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

fn load(p: vec2<i32>) -> vec3<f32> {
    let last = vec2<i32>(params.in_size) - vec2<i32>(1);
    let c = textureLoad(src, clamp(p, vec2<i32>(0), last), 0).rgb;
    return clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
}

// Simplest multi-channel approximate luma possible (luma times 2).
fn luma(c: vec3<f32>) -> f32 {
    return c.b * 0.5 + (c.r * 0.5 + c.g);
}

// Approximations from ffx_a.h (APrxLoRcpF1, APrxLoRsqF1): integer tricks on the float bits. They are
// finite at zero, which EASU relies on for flat areas.
fn rcp_lo(a: f32) -> f32 {
    return bitcast<f32>(0x7ef07ebbu - bitcast<u32>(a));
}

fn rsq_lo(a: f32) -> f32 {
    return bitcast<f32>(0x5f347d74u - (bitcast<u32>(a) >> 1u));
}

// Filtering for one tap: returns the weighted colour in xyz and the weight in w.
fn easu_tap(off: vec2<f32>, dir: vec2<f32>, len: vec2<f32>, lob: f32, clp: f32, c: vec3<f32>) -> vec4<f32> {
    // Rotate the offset by the direction, then apply the anisotropy.
    var v = vec2<f32>(off.x * dir.x + off.y * dir.y, off.x * (-dir.y) + off.y * dir.x);
    v = v * len;
    // Limit to the window: at a corner two taps can easily be outside.
    let d2 = min(v.x * v.x + v.y * v.y, clp);
    // Approximation of lanczos2 without sin(), rcp() or sqrt():
    //  (25/16 * (2/5 * x^2 - 1)^2 - (25/16 - 1)) * (1/4 * x^2 - 1)^2
    var w_b = (2.0 / 5.0) * d2 - 1.0;
    var w_a = lob * d2 - 1.0;
    w_b = w_b * w_b;
    w_a = w_a * w_a;
    w_b = (25.0 / 16.0) * w_b - (25.0 / 16.0 - 1.0);
    let w = w_b * w_a;
    return vec4<f32>(c * w, w);
}

// Accumulate the direction (xy) and the length (z) around one bilinear corner of weight `w`.
//    a
//  b c d
//    e
fn easu_set(w: f32, l_a: f32, l_b: f32, l_c: f32, l_d: f32, l_e: f32) -> vec3<f32> {
    // The direction is the '+' difference; the magnitude comes from the average of both sides of 'c'.
    // The length takes a gradient reversal to 0, smoothly to non-reversal at 1, shaped.
    let dir_x = l_d - l_b;
    var len_x = rcp_lo(max(abs(l_d - l_c), abs(l_c - l_b)));
    len_x = saturate(abs(dir_x) * len_x);
    len_x = len_x * len_x;
    let dir_y = l_e - l_a;
    var len_y = rcp_lo(max(abs(l_e - l_c), abs(l_c - l_a)));
    len_y = saturate(abs(dir_y) * len_y);
    len_y = len_y * len_y;
    return vec3<f32>(dir_x * w, dir_y * w, (len_x + len_y) * w);
}

@fragment
fn fs_easu(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    // The output pixel position in input pixels, and the position of 'f'.
    let ratio = params.in_size / params.out_size;
    var pp = floor(frag.xy) * ratio + (0.5 * ratio - vec2<f32>(0.5));
    let fp = floor(pp);
    pp = pp - fp;
    let o = vec2<i32>(fp);

    // 12-tap kernel.
    //    b c
    //  e f g h
    //  i j k l
    //    n o
    let b = load(o + vec2<i32>(0, -1));
    let c = load(o + vec2<i32>(1, -1));
    let e = load(o + vec2<i32>(-1, 0));
    let f = load(o);
    let g = load(o + vec2<i32>(1, 0));
    let h = load(o + vec2<i32>(2, 0));
    let i = load(o + vec2<i32>(-1, 1));
    let j = load(o + vec2<i32>(0, 1));
    let k = load(o + vec2<i32>(1, 1));
    let l = load(o + vec2<i32>(2, 1));
    let n = load(o + vec2<i32>(0, 2));
    let p = load(o + vec2<i32>(1, 2));
    let b_l = luma(b);
    let c_l = luma(c);
    let e_l = luma(e);
    let f_l = luma(f);
    let g_l = luma(g);
    let h_l = luma(h);
    let i_l = luma(i);
    let j_l = luma(j);
    let k_l = luma(k);
    let l_l = luma(l);
    let n_l = luma(n);
    let p_l = luma(p);

    // Accumulate for bilinear interpolation (s t / u v).
    var dl = easu_set((1.0 - pp.x) * (1.0 - pp.y), b_l, e_l, f_l, g_l, j_l);
    dl = dl + easu_set(pp.x * (1.0 - pp.y), c_l, f_l, g_l, h_l, k_l);
    dl = dl + easu_set((1.0 - pp.x) * pp.y, f_l, i_l, j_l, k_l, n_l);
    dl = dl + easu_set(pp.x * pp.y, g_l, j_l, k_l, l_l, p_l);

    // Normalize with the approximation, and clean up close to zero.
    var dir = dl.xy;
    let dir2 = dir * dir;
    var dir_r = dir2.x + dir2.y;
    let zro = dir_r < (1.0 / 32768.0);
    dir_r = select(rsq_lo(dir_r), 1.0, zro);
    dir.x = select(dir.x, 1.0, zro);
    dir = dir * dir_r;
    // Transform from {0 to 2} to {0 to 1} range, and shape with a square.
    var len = dl.z * 0.5;
    len = len * len;
    // Stretch the kernel: 1.0 vertical or horizontal, to sqrt(2.0) on a diagonal.
    let stretch = (dir.x * dir.x + dir.y * dir.y) * rcp_lo(max(abs(dir.x), abs(dir.y)));
    // Anisotropic length after rotation: x goes from 1.0 to 'stretch' on edges, y from 1.0 to 2x.
    let len2 = vec2<f32>(1.0 + (stretch - 1.0) * len, 1.0 - 0.5 * len);
    // Based on the amount of 'edge', the window shifts from +/-{sqrt(2.0) to slightly beyond 2.0}.
    let lob = 0.5 + ((1.0 / 4.0 - 0.04) - 0.5) * len;
    // The distance^2 clipping point is the end of the adjustable window.
    let clp = rcp_lo(lob);

    // Min and max of the 4 nearest taps, for the deringing.
    let min4 = min(min(f, g), min(j, k));
    let max4 = max(max(f, g), max(j, k));

    var acc = easu_tap(vec2<f32>(0.0, -1.0) - pp, dir, len2, lob, clp, b);
    acc = acc + easu_tap(vec2<f32>(1.0, -1.0) - pp, dir, len2, lob, clp, c);
    acc = acc + easu_tap(vec2<f32>(-1.0, 1.0) - pp, dir, len2, lob, clp, i);
    acc = acc + easu_tap(vec2<f32>(0.0, 1.0) - pp, dir, len2, lob, clp, j);
    acc = acc + easu_tap(vec2<f32>(0.0, 0.0) - pp, dir, len2, lob, clp, f);
    acc = acc + easu_tap(vec2<f32>(-1.0, 0.0) - pp, dir, len2, lob, clp, e);
    acc = acc + easu_tap(vec2<f32>(1.0, 1.0) - pp, dir, len2, lob, clp, k);
    acc = acc + easu_tap(vec2<f32>(2.0, 1.0) - pp, dir, len2, lob, clp, l);
    acc = acc + easu_tap(vec2<f32>(2.0, 0.0) - pp, dir, len2, lob, clp, h);
    acc = acc + easu_tap(vec2<f32>(1.0, 0.0) - pp, dir, len2, lob, clp, g);
    acc = acc + easu_tap(vec2<f32>(1.0, 2.0) - pp, dir, len2, lob, clp, p);
    acc = acc + easu_tap(vec2<f32>(0.0, 2.0) - pp, dir, len2, lob, clp, n);

    // Normalize and dering.
    let rgb = min(max4, max(min4, acc.rgb * (1.0 / acc.w)));
    return vec4<f32>(rgb, 1.0);
}

// This is set at the limit of providing unnatural results for sharpening.
const RCAS_LIMIT: f32 = 0.25 - (1.0 / 16.0);
// Keeps the limiter divisions finite on pure black and pure white neighbourhoods.
const RCAS_EPSILON: f32 = 1.0 / 65536.0;

@fragment
fn fs_rcas(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    // The algorithm uses a minimal 3x3 neighbourhood.
    //    b
    //  d e f
    //    h
    let sp = vec2<i32>(frag.xy);
    let b = load(sp + vec2<i32>(0, -1));
    let d = load(sp + vec2<i32>(-1, 0));
    let e = load(sp);
    let f = load(sp + vec2<i32>(1, 0));
    let h = load(sp + vec2<i32>(0, 1));

    // Min and max of the ring.
    let mn4 = min(min(b, d), min(f, h));
    let mx4 = max(max(b, d), max(f, h));
    // Limiters.
    let hit_min = min(mn4, e) / (4.0 * max(mx4, vec3<f32>(RCAS_EPSILON)));
    let hit_max = (vec3<f32>(1.0) - max(mx4, e)) / min(4.0 * mn4 - vec3<f32>(4.0), vec3<f32>(-RCAS_EPSILON));
    let lobe_rgb = max(-hit_min, hit_max);
    let lobe_max = max(lobe_rgb.r, max(lobe_rgb.g, lobe_rgb.b));
    let lobe = max(-RCAS_LIMIT, min(lobe_max, 0.0)) * params.rcas;

    // Resolve.
    let rgb = (lobe * b + lobe * d + lobe * h + lobe * f + e) / (4.0 * lobe + 1.0);
    return vec4<f32>(rgb, 1.0);
}
