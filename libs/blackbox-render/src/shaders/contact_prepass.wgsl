@group(0) @binding(0) var<uniform> camera: mat4x4<f32>;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
struct In {
    @location(0) position: vec3<f32>, @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>, @location(3) uv: vec2<f32>,
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
struct Out { @invariant @builtin(position) clip: vec4<f32>, @location(0) normal: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) alpha: f32, @location(3) world: vec3<f32>, };
@vertex fn vs_main(v: In, placed: InstanceIn) -> Out {
    var out: Out;
    let model = instance_model(placed);
    let world = model * vec4<f32>(v.position, 1.0);
    out.clip = camera * world;
    out.world = world.xyz;
    out.normal = cross(model[1].xyz, model[2].xyz) * v.normal.x + cross(model[2].xyz, model[0].xyz) * v.normal.y + cross(model[0].xyz, model[1].xyz) * v.normal.z;
    out.uv = v.uv; out.alpha = v.color.a;
    return out;
}
fn encoded(n: vec3<f32>) -> vec4<f32> {
    if length(n) < 0.001 { return vec4<f32>(0.0); }
    return vec4<f32>(normalize(n) * 0.5 + 0.5, 1.0);
}
@fragment fn fs_opaque(in: Out) -> @location(0) vec4<f32> { return encoded(in.normal); }
@fragment fn fs_cutout(in: Out) -> @location(0) vec4<f32> {
    if textureSample(image, image_sampler, in.uv).a * in.alpha < 0.5 { discard; }
    return encoded(in.normal);
}
@fragment fn fs_glossy_cutout(in: Out) -> @location(0) vec4<f32> {
    // Match the glossy colour shader: vertex alpha is not the texture-alpha cutoff.
    if textureSample(image, image_sampler, in.uv).a < 0.5 { discard; }
    return encoded(in.normal);
}

@fragment fn fs_tree_cutout(in: Out) -> @location(0) vec4<f32> {
    // Leaf lighting normals are deliberately bent away from their card planes.
    // AO/contact rays need the visible geometric plane; evaluate before discard.
    let plane = -cross(dpdx(in.world), dpdy(in.world));
    if textureSample(image, image_sampler, in.uv).a < 0.5 { discard; }
    return encoded(plane);
}

@fragment fn fs_prelit_cutout(in: Out) -> @location(0) vec4<f32> {
    if 2.0 * textureSample(image, image_sampler, in.uv).a * in.alpha < 0.5 { discard; }
    return encoded(in.normal);
}
