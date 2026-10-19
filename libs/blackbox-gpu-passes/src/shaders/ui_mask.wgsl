// Compose at the source picture's texel centres, then use the ordinary filtered UI texture.
// This preserves the CPU path's two RGBA8 rounding steps and gamma-space premultiplication.
struct Params {
    window: vec4<f32>,
    pivot: vec2<f32>,
    sin_cos: vec2<f32>,
    additive: vec4<u32>,
};
@group(0) @binding(0) var picture: texture_2d<f32>;
@group(0) @binding(1) var mask: texture_2d<f32>;
@group(0) @binding(2) var output_image: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var<uniform> params: Params;

fn alpha_at(position: vec2<f32>) -> f32 {
    let extent = vec2<i32>(textureDimensions(mask));
    if any(position < vec2<f32>(0.0)) || any(position > vec2<f32>(extent)) {
        return 0.0;
    }
    let f = position - vec2<f32>(0.5);
    let lo = vec2<i32>(floor(f));
    let weight = f - floor(f);
    let hi = extent - vec2<i32>(1);
    let a = textureLoad(mask, clamp(lo, vec2<i32>(0), hi), 0).a;
    let b = textureLoad(mask, clamp(lo + vec2<i32>(1, 0), vec2<i32>(0), hi), 0).a;
    let c = textureLoad(mask, clamp(lo + vec2<i32>(0, 1), vec2<i32>(0), hi), 0).a;
    let d = textureLoad(mask, clamp(lo + vec2<i32>(1, 1), vec2<i32>(0), hi), 0).a;
    let top = a * (1.0 - weight.x) + b * weight.x;
    let bottom = c * (1.0 - weight.x) + d * weight.x;
    return top * (1.0 - weight.y) + bottom * weight.y;
}

@compute @workgroup_size(8, 8)
fn compose(@builtin(global_invocation_id) index: vec3<u32>) {
    let size = textureDimensions(picture);
    if any(index.xy >= size) { return; }
    let extent = vec2<f32>(textureDimensions(mask));
    let centre = extent * 0.5;
    let pivot = extent * params.pivot - centre;
    let uv = params.window.xy + (vec2<f32>(index.xy) + 0.5) / vec2<f32>(size)
        * (params.window.zw - params.window.xy);
    let source = uv * extent - centre - pivot;
    let sin = params.sin_cos.x;
    let cos = params.sin_cos.y;
    let position = vec2<f32>(source.x * cos + source.y * sin,
        source.y * cos - source.x * sin) + pivot + centre;
    let pixel = vec4<u32>(round(textureLoad(picture, vec2<i32>(index.xy), 0) * 255.0));
    let alpha = u32(f32(pixel.a) * alpha_at(position) + 0.5);
    let rgb = (pixel.rgb * alpha + vec3<u32>(127)) / vec3<u32>(255);
    let result = vec4<u32>(rgb, select(alpha, 0u, params.additive.x != 0u));
    textureStore(output_image, vec2<i32>(index.xy), vec4<f32>(result) / 255.0);
}
