// A scene grade of the finished frame (`FocusFilter::grade`): the colour multiplier, the contrast about mid grey and a
// darkening vignette. params.a is (edge colour, inner distance), params.b is (tint, contrast). The edge colour is what
// the frame is multiplied by at a corner of the screen, reached smoothly from the inner distance (0 centre, 1 corner).
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let original = textureLoad(src, vec2<i32>(in.clip.xy), 0);
    if all(params.a.xyz == vec3<f32>(1.0)) && all(params.b == vec4<f32>(1.0)) { return original; }
    var rgb = original.rgb * params.b.xyz;
    rgb = (rgb - vec3<f32>(0.5)) * params.b.w + vec3<f32>(0.5);
    let reach = length(in.uv - vec2<f32>(0.5)) * 1.41421356;
    let edge = smoothstep(params.a.w, 1.0, reach);
    rgb = rgb * mix(vec3<f32>(1.0), params.a.xyz, edge);
    return vec4<f32>(max(rgb, vec3<f32>(0.0)), original.a);
}
