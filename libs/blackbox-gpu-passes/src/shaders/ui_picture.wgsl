// A view capture shown as a UI texture: the colour times a per-channel tint, then times the mask's alpha, with the
// alpha of the result being the mask's. The same integer rounding as the CPU path, so the two agree to the bit.
struct Params {
    tint: vec4<u32>,
};
@group(0) @binding(0) var picture: texture_2d<f32>;
@group(0) @binding(1) var mask: texture_2d<f32>;
@group(0) @binding(2) var output_image: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(8, 8)
fn compose(@builtin(global_invocation_id) index: vec3<u32>) {
    let size = textureDimensions(picture);
    if any(index.xy >= size) { return; }
    let at = vec2<i32>(index.xy);
    let rgb = vec3<u32>(round(textureLoad(picture, at, 0).rgb * 255.0));
    let cover = u32(round(textureLoad(mask, at, 0).a * 255.0));
    let tinted = (rgb * params.tint.xyz + vec3<u32>(127u)) / vec3<u32>(255u);
    let shaded = (tinted * cover + vec3<u32>(127u)) / vec3<u32>(255u);
    textureStore(output_image, at, vec4<f32>(vec4<u32>(shaded, cover)) / 255.0);
}
