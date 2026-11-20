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
        if (bright && params.b.y <= 0.0) {
            c = threshold(c);
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
fn fs_up(in: VsOut) -> @location(0) vec4<f32> {
    let d = 1.0 / vec2<f32>(textureDimensions(src));
    var sum = tap(src, in.uv) * 4.0;
    sum += (tap(src, in.uv + vec2<f32>(-d.x, 0.0)) + tap(src, in.uv + vec2<f32>(d.x, 0.0))
        + tap(src, in.uv + vec2<f32>(0.0, -d.y)) + tap(src, in.uv + vec2<f32>(0.0, d.y))) * 2.0;
    sum += tap(src, in.uv + vec2<f32>(-d.x, -d.y)) + tap(src, in.uv + vec2<f32>(d.x, -d.y))
        + tap(src, in.uv + vec2<f32>(-d.x, d.y)) + tap(src, in.uv + vec2<f32>(d.x, d.y));
    return vec4<f32>(sum / 16.0, 0.0);
}

fn flare_sample(uv: vec2<f32>) -> vec3<f32> {
    let edge = min(uv, 1.0-uv);
    let fade = smoothstep(0.0, 0.08, min(edge.x,edge.y));
    let c = textureSampleLevel(src2, samp, uv, 0.0).rgb * params.b.w;
    return max(c - vec3<f32>(select(0.0, 0.8, params.b.y > 0.0)), vec3<f32>(0.0)) * fade;
}

fn optics(base: vec3<f32>, glow: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    var lit = base + glow * params.a.z;
    if params.b.y > 0.0 {
        lit = mix(base, glow * params.b.w, 0.12 * params.b.y);
    }
    if params.b.z <= 0.0 { return lit; }
    let axis = uv - 0.5;
    let direction = axis / max(length(axis), 0.0001);
    let ghost = flare_sample(0.5 - axis * 1.4) + flare_sample(0.5 - axis * 0.65);
    let ring = uv - direction * 0.32;
    let halo = vec3<f32>(flare_sample(ring + direction*0.003).r, flare_sample(ring).g, flare_sample(ring-direction*0.003).b);
    return lit + (ghost + halo) * (0.04 * params.b.z);
}

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    let base = textureLoad(src, vec2<i32>(in.clip.xy), 0);
    let glow = textureSampleLevel(src2, samp, in.uv, 0.0).rgb;
    return vec4<f32>(optics(base.rgb, glow, in.uv), 1.0);
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
    let lit = optics(base.rgb, glow, in.uv);
    return vec4<f32>(aces(max(lit, vec3<f32>(0.0)) * params.b.x), 1.0);
}
