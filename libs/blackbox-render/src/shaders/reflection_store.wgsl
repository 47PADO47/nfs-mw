// Preserve display-encoded bytes; RH images need a vertical flip for cube addressing.
@group(0) @binding(0) var color: texture_2d<f32>;
@group(0) @binding(1) var depths: texture_depth_2d_array;
struct Control { color_far: vec4<f32>, face: vec4<f32>, };
@group(0) @binding(2) var<uniform> control: Control;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}
@fragment fn fs(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(color));
    let pixel = vec2<i32>(i32(position.x), size.y - 1 - i32(position.y));
    let value = textureLoad(color, pixel, 0);
    if control.color_far.w <= 0.0 { return value; }
    let depth = textureLoad(depths, pixel, i32(control.face.x), 0);
    if depth <= 0.0 { return value; }
    let xy = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(size) * 2.0 - 1.0;
    let radius = 0.25 / depth * sqrt(1.0 + dot(xy, xy));
    let fade = smoothstep(control.color_far.w * 0.85, control.color_far.w, radius);
    return vec4<f32>(mix(value.rgb, control.color_far.rgb, fade), value.a);
}
