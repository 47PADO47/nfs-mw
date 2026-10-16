// Radial blur. Each pixel averages `taps` samples on the segment from itself to the blur centre. The first
// sample is the pixel; the sample at position t (0 at the pixel, 1 at the far end) weighs 1 - t / 2, so the pixel
// counts most. The reach grows from 0 at the inner radius to the strength at the farthest point.
// Params (see `RadialBlurSettings` in radial_blur.rs):
//   a.x strength (0 to 1), a.y inner radius in UV, a.z tap count, a.w 1 / total tap weight
//   b.xy blur centre in UV

const MAX_TAPS: u32 = 32u;

@fragment
fn fs_radial(in: VsOut) -> @location(0) vec4<f32> {
    let offset = in.uv - params.b.xy;
    let reach = params.a.x * clamp((length(offset) - params.a.y) / (1.0 - params.a.y), 0.0, 1.0);
    if reach <= 0.0 {
        return textureSampleLevel(src, samp, in.uv, 0.0);
    }
    let taps = min(u32(params.a.z), MAX_TAPS);
    let span = f32(max(taps, 2u) - 1u);
    var sum = vec4<f32>(0.0);
    for (var i = 0u; i < taps; i++) {
        let t = f32(i) / span;
        sum += (1.0 - 0.5 * t) * textureSampleLevel(src, samp, in.uv - offset * reach * t, 0.0);
    }
    return sum * params.a.w;
}
