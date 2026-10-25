@group(0) @binding(0) var<uniform> light_matrix: mat4x4<f32>;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
struct In {
    @location(0) position: vec3<f32>, @location(2) color: vec4<f32>, @location(3) uv: vec2<f32>,
};
// instance input: begin
// The per-instance model matrix, one column per attribute. A renderer with an instance store replaces this block
// (see `instance_view.rs`): the instance then carries a slot at location 4 and the matrix is read from the store.
struct InstanceIn {
    @location(4) model_0: vec4<f32>, @location(5) model_1: vec4<f32>,
    @location(6) model_2: vec4<f32>, @location(7) model_3: vec4<f32>,
    @location(12) shadow_flags: vec4<f32>,
};
fn instance_model(i: InstanceIn) -> mat4x4<f32> { return mat4x4<f32>(i.model_0, i.model_1, i.model_2, i.model_3); }
fn instance_receives_shadow(i: InstanceIn) -> u32 { return u32(i.shadow_flags.x); }
// instance input: end
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) alpha: f32, };
@vertex fn vs_main(v: In, placed: InstanceIn) -> Out {
    var out: Out;
    out.position = light_matrix * instance_model(placed) * vec4<f32>(v.position, 1.0);
    out.uv = v.uv;
    out.alpha = v.color.a;
    return out;
}
@fragment fn fs_opaque() {}
@fragment fn fs_cutout(in: Out) {
    if textureSample(image, image_sampler, in.uv).a * in.alpha < 0.5 { discard; }
}

@fragment fn fs_tree_cutout(in: Out) {
    if textureSample(image, image_sampler, in.uv).a < 0.5 { discard; }
}
@fragment fn fs_prelit_cutout(in: Out) {
    if 2.0 * textureSample(image, image_sampler, in.uv).a * in.alpha <= 0.375 { discard; }
}

@fragment fn fs_world_cutout(in: Out) {
    if textureSample(image, image_sampler, in.uv).a * in.alpha <= 0.375 { discard; }
}
