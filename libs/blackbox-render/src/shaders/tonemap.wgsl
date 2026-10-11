// Filmic tone mapping: exposure, then an ACES fit. params.a.x is the exposure.
//
// The curve is Krzysztof Narkowicz's rational fit of the ACES RRT+ODT ("ACES Filmic Tone Mapping
// Curve", 2016). It maps 1.0 to about 0.80, so it darkens an image that was authored for a clamp.

fn aces(x: vec3<f32>) -> vec3<f32> {
    let mapped = (x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14);
    return clamp(mapped, vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(src, vec2<i32>(in.clip.xy), 0);
    return vec4<f32>(aces(max(c.rgb, vec3<f32>(0.0)) * params.a.x), c.a);
}
