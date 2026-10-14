// Generic additive screen-mask contribution: green addition, blue gain.
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let original = textureLoad(src, vec2<i32>(in.clip.xy), 0);
    if params.a.x <= 0.0 {
        return original;
    }
    let mask = textureSampleLevel(src2, samp, vec2<f32>(in.uv.x, 1.0 - in.uv.y), 0.0);
    return vec4<f32>(original.rgb + vec3<f32>(mask.g * params.a.x * mask.b), original.a);
}
