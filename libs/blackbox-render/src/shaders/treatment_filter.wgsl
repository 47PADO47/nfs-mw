// Local luminance and four-tap blur; docs/specs/visual-treatment.md.
fn extract(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(dot(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)), params.b.xyz));
}

fn down(uv: vec2<f32>) -> vec4<f32> {
    let step = 0.5 / vec2<f32>(textureDimensions(src));
    let a = extract(textureSampleLevel(src, samp, uv + step * vec2<f32>(-1.0, -1.0), 0.0).rgb);
    let b = extract(textureSampleLevel(src, samp, uv + step * vec2<f32>( 1.0, -1.0), 0.0).rgb);
    let c = extract(textureSampleLevel(src, samp, uv + step * vec2<f32>(-1.0,  1.0), 0.0).rgb);
    let d = extract(textureSampleLevel(src, samp, uv + step * vec2<f32>( 1.0,  1.0), 0.0).rgb);
    return vec4<f32>((a + b + c + d) * 0.25, 1.0);
}

@fragment fn fs_luma(in: VsOut) -> @location(0) vec4<f32> { return down(in.uv); }

fn blur(uv: vec2<f32>, axis: vec2<f32>) -> vec4<f32> {
    let step = axis / vec2<f32>(textureDimensions(src));
    return (textureSampleLevel(src, samp, uv - step * 1.5, 0.0)
          + textureSampleLevel(src, samp, uv + step * 1.5, 0.0)) / 3.0
         + (textureSampleLevel(src, samp, uv - step * 3.0, 0.0)
          + textureSampleLevel(src, samp, uv + step * 3.0, 0.0)) / 6.0;
}
@fragment fn fs_horizontal(in: VsOut) -> @location(0) vec4<f32> { return blur(in.uv, vec2<f32>(1.0, 0.0)); }
@fragment fn fs_vertical(in: VsOut) -> @location(0) vec4<f32> { return blur(in.uv, vec2<f32>(0.0, 1.0)); }
