// Bloom: bright areas are blurred through a chain of half-size images and added back.
//
// params.a = (threshold, knee, intensity, 0).
//   fs_prefilter  scene -> mip 0: keep what is above the threshold (soft knee), 13-tap downsample
//   fs_down       mip i -> mip i+1: 13-tap downsample (the "dual filter" of the 2014 Call of Duty talk)
//   fs_up         mip i+1 -> mip i, blended additively: 3x3 tent upsample
//   fs_composite  scene + mip 0 * intensity -> output

// Single taps are capped so one very bright texel cannot flash over the whole screen.
const MAX_TAP: f32 = 32.0;

fn tap(tex: texture_2d<f32>, uv: vec2<f32>) -> vec3<f32> {
    return min(textureSampleLevel(tex, samp, uv, 0.0).rgb, vec3<f32>(MAX_TAP));
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
        if (bright) {
            c = threshold(c);
        }
        t[i] = c;
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

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    let base = textureLoad(src, vec2<i32>(in.clip.xy), 0);
    let glow = textureSampleLevel(src2, samp, in.uv, 0.0).rgb;
    return vec4<f32>(base.rgb + glow * params.a.z, base.a);
}
