@group(0) @binding(0) var<uniform> camera: mat4x4<f32>;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
struct In {
    @location(0) position: vec3<f32>, @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>, @location(3) uv: vec2<f32>,
    @location(4) m0: vec4<f32>, @location(5) m1: vec4<f32>, @location(6) m2: vec4<f32>, @location(7) m3: vec4<f32>,
};
struct Out { @invariant @builtin(position) clip: vec4<f32>, @location(0) normal: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) alpha: f32, @location(3) world: vec3<f32>, };
@vertex fn vs_main(v: In) -> Out {
    var out: Out;
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    let world = model * vec4<f32>(v.position, 1.0);
    out.clip = camera * world;
    out.world = world.xyz;
    out.normal = cross(v.m1.xyz, v.m2.xyz) * v.normal.x + cross(v.m2.xyz, v.m0.xyz) * v.normal.y + cross(v.m0.xyz, v.m1.xyz) * v.normal.z;
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
