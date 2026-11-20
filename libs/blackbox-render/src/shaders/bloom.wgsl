// Five-group first downsample adapted from Bevy 0.20, Bevy contributors.
// MIT License
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
// Bloom: bright areas are blurred through a chain of half-size images and added back.
//
// params.a = (threshold, knee, intensity, 0).
//   fs_prefilter  scene -> mip 0: keep what is above the threshold (soft knee), 13-tap downsample
//   fs_down       mip i -> mip i+1: 13-tap downsample (the "dual filter" of the 2014 Call of Duty talk)
//   fs_up         mip i+1 -> mip i, blended additively: 3x3 tent upsample
//   fs_composite  scene + mip 0 * intensity -> output
//   fs_composite_tonemap  the same, then the tone mapping of tonemap.wgsl (params.b.x is the exposure): the two
//                 passes fused into one

// Single taps are capped so one very bright texel cannot flash over the whole screen.
const MAX_TAP: f32 = 32.0;

fn tap(tex: texture_2d<f32>, uv: vec2<f32>) -> vec3<f32> {
    let c = textureSampleLevel(tex, samp, uv, 0.0).rgb;
    if params.b.y > 0.0 { return min(c, vec3<f32>(8192.0)); }
    return min(c, vec3<f32>(MAX_TAP));
}

fn threshold(c: vec3<f32>) -> vec3<f32> {
    let limit = params.a.x;
    let knee = max(params.a.y, 1e-4);
    let brightness = max(c.r, max(c.g, c.b));
    let soft = clamp(brightness - limit + knee, 0.0, 2.0 * knee);
    let weight = max(soft * soft / (4.0 * knee), brightness - limit) / max(brightness, 1e-4);
    return c * max(weight, 0.0);
}

// The native scene stores display-encoded RGB even in its floating-point target.
fn bloom_linear(c: vec3<f32>) -> vec3<f32> {
    let rgb = max(c, vec3<f32>(0.0));
    return select(pow((rgb + 0.055) / 1.055, vec3<f32>(2.4)), rgb / 12.92, rgb <= vec3<f32>(0.04045));
}

fn bloom_encoded(c: vec3<f32>) -> vec3<f32> {
    let rgb = max(c, vec3<f32>(0.0));
    return select(1.055 * pow(rgb, vec3<f32>(1.0 / 2.4)) - 0.055, rgb * 12.92, rgb <= vec3<f32>(0.0031308));
}

// Limit the scattered energy before the six levels accumulate into an FP16 target.
fn scatter_source(linear: vec3<f32>) -> vec3<f32> {
    let colour = min(linear, vec3<f32>(8192.0));
    return colour * smoothstep(0.15, 0.65, max(colour.r, max(colour.g, colour.b)));
}

// The 13 taps: a centre, four at +-1 texel on the diagonals and eight at +-2 texels, weighted so
// the five overlapping 2x2 boxes sum to one. `bright` applies the threshold to each tap.
fn down13(tex: texture_2d<f32>, uv: vec2<f32>, bright: bool) -> vec3<f32> {
    let d = 1.0 / vec2<f32>(textureDimensions(tex));
    var t: array<vec3<f32>, 13>;
    var o = array<vec2<f32>, 13>(
        vec2<f32>(-2.0, -2.0), vec2<f32>(0.0, -2.0), vec2<f32>(2.0, -2.0),
        vec2<f32>(-2.0, 0.0), vec2<f32>(0.0, 0.0), vec2<f32>(2.0, 0.0),
        vec2<f32>(-2.0, 2.0), vec2<f32>(0.0, 2.0), vec2<f32>(2.0, 2.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, 1.0),
    );
    for (var i = 0; i < 13; i++) {
        var c = tap(tex, uv + o[i] * d);
        if bright {
            if params.b.y > 0.0 {
                c = scatter_source(bloom_linear(c));
            } else {
                c = threshold(c);
            }
        }
        t[i] = c;
    }
    if bright && params.b.y > 0.0 {
        let groups = array(
            t[0]+t[1]+t[3]+t[4], t[1]+t[2]+t[4]+t[5],
            t[3]+t[4]+t[6]+t[7], t[4]+t[5]+t[7]+t[8], t[9]+t[10]+t[11]+t[12]);
        var sum = vec3<f32>(0.0);
        var total = 0.0;
        for (var i = 0; i < 5; i++) {
            let colour = groups[i] * 0.25;
            let weight = select(0.125, 0.5, i == 4) / (1.0 + dot(colour, vec3<f32>(0.2126,0.7152,0.0722)));
            sum += colour * weight;
            total += weight;
        }
        return sum / total;
    }
    let corners = t[0] + t[2] + t[6] + t[8];
    let edges = t[1] + t[3] + t[5] + t[7];
    let inner = t[9] + t[10] + t[11] + t[12];
    return t[4] * 0.125 + corners * 0.03125 + edges * 0.0625 + inner * 0.125;
}

@fragment
fn fs_prefilter(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(down13(src, in.uv, true), 1.0);
}

@fragment
fn fs_down(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(down13(src, in.uv, false), 1.0);
}

@fragment
fn fs_physical_up(in: VsOut) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(src));
    let d = 1.0 / size;
    // Fold the separable [1,2,1] tent into four bilinear reads. Account for
    // fractional texel centres even when a mip has an odd dimension.
    let p = in.uv * size - 0.5;
    let f = fract(p);
    let right = 1.0 + 2.0 * f;
    let left = 4.0 - right;
    let centre = floor(p) + 0.5;
    let lo = (centre - 1.0 + (2.0 - f) / left) * d;
    let hi = (centre + 1.0 + f / right) * d;
    let a = mix(tap(src, lo), tap(src, vec2(hi.x, lo.y)), right.x * 0.25);
    let b = mix(tap(src, vec2(lo.x, hi.y)), tap(src, hi), right.x * 0.25);
    return vec4<f32>(mix(a, b, right.y * 0.25), 0.0);
}

@fragment
fn fs_up(in: VsOut) -> @location(0) vec4<f32> {
    let d = 1.0 / vec2<f32>(textureDimensions(src));
    var sum = tap(src, in.uv) * 4.0;
    sum += (tap(src, in.uv + vec2<f32>(-d.x, 0.0)) + tap(src, in.uv + vec2<f32>(d.x, 0.0))
        + tap(src, in.uv + vec2<f32>(0.0, -d.y)) + tap(src, in.uv + vec2<f32>(0.0, d.y))) * 2.0;
    sum += tap(src, in.uv + vec2<f32>(-d.x, -d.y)) + tap(src, in.uv + vec2<f32>(d.x, -d.y))
        + tap(src, in.uv + vec2<f32>(-d.x, d.y)) + tap(src, in.uv + vec2<f32>(d.x, d.y));
    return vec4<f32>(sum / 16.0, 0.0);
}

fn composite_bloom(base: vec3<f32>, glow: vec3<f32>) -> vec3<f32> {
    if params.b.y <= 0.0 { return base + glow * params.a.z; }
    let linear = bloom_linear(base);
    let scatter = glow * params.b.w - scatter_source(linear);
    return bloom_encoded(linear + scatter * (0.45 * params.b.y));
}

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    let base = textureLoad(src, vec2<i32>(in.clip.xy), 0);
    let glow = textureSampleLevel(src2, samp, in.uv, 0.0).rgb;
    return vec4<f32>(composite_bloom(base.rgb, glow), 1.0);
}

// The ACES fit of tonemap.wgsl, kept identical to it (a test compares the two texts).
fn aces(x: vec3<f32>) -> vec3<f32> {
    let mapped = (x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14);
    return clamp(mapped, vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_composite_tonemap(in: VsOut) -> @location(0) vec4<f32> {
    let base = textureLoad(src, vec2<i32>(in.clip.xy), 0);
    let glow = textureSampleLevel(src2, samp, in.uv, 0.0).rgb;
    let lit = composite_bloom(base.rgb, glow);
    return vec4<f32>(aces(max(lit, vec3<f32>(0.0)) * params.b.x), 1.0);
}
