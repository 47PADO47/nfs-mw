// Motion vectors for the temporal upscaler: where each opaque surface point was in the previous frame, minus
// where it is now, in normalised device coordinates (y up). Drawn after the scene, against its finished depth
// buffer, so each pixel gets the vector of the surface the colour pass left visible.

struct Camera {
    // World to clip with this frame's sub-pixel jitter: the position the colour pass drew at.
    jittered: mat4x4<f32>,
    // World to clip without jitter, this frame and the last: the vectors must not contain the jitter.
    current: mat4x4<f32>,
    previous: mat4x4<f32>,
    // x = texture LOD bias of the colour pass, so alpha-tested edges are cut the same way.
    bias: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;

struct In {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
    // Last frame's model matrix, one column per attribute (this frame's is the instance input).
    @location(8) p0: vec4<f32>, @location(9) p1: vec4<f32>, @location(10) p2: vec4<f32>, @location(11) p3: vec4<f32>,
};

// instance input: begin
// The per-instance model matrix, one column per attribute. A renderer with an instance store replaces this block
// (see `instance_view.rs`): the instance then carries a slot at location 4 and the matrix is read from the store.
struct InstanceIn {
    @location(4) model_0: vec4<f32>, @location(5) model_1: vec4<f32>,
    @location(6) model_2: vec4<f32>, @location(7) model_3: vec4<f32>,
};
fn instance_model(i: InstanceIn) -> mat4x4<f32> { return mat4x4<f32>(i.model_0, i.model_1, i.model_2, i.model_3); }
// instance input: end

struct Out {
    // Same expression as the scene shader's, so the depth matches the colour pass's exactly.
    @invariant @builtin(position) clip: vec4<f32>,
    @location(0) current: vec4<f32>,
    @location(1) previous: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) alpha: f32,
};

@vertex
fn vs_main(v: In, placed: InstanceIn) -> Out {
    let model = instance_model(placed);
    let before = mat4x4<f32>(v.p0, v.p1, v.p2, v.p3);
    let world = model * vec4<f32>(v.position, 1.0);
    var out: Out;
    out.clip = camera.jittered * world;
    out.current = camera.current * world;
    out.previous = camera.previous * (before * vec4<f32>(v.position, 1.0));
    out.uv = v.uv;
    out.alpha = v.color.a;
    return out;
}

// A point that was behind the camera last frame has no earlier position on screen: no motion.
fn motion_vector(in: Out) -> vec4<f32> {
    if in.previous.w <= 1e-6 || in.current.w <= 1e-6 {
        return vec4<f32>(0.0);
    }
    let motion = in.previous.xy / in.previous.w - in.current.xy / in.current.w;
    // RG motion targets discard Z/W. AO's RGBA target also needs the old object-space
    // point's view depth, rather than predicting its depth from camera motion alone.
    return vec4<f32>(motion, in.previous.w, 1.0);
}

@fragment
fn fs_opaque(in: Out) -> @location(0) vec4<f32> {
    return motion_vector(in);
}

@fragment
fn fs_cutout(in: Out) -> @location(0) vec4<f32> {
    if textureSampleBias(image, image_sampler, in.uv, camera.bias.x).a * in.alpha < 0.5 { discard; }
    return motion_vector(in);
}

@fragment
fn fs_glossy_cutout(in: Out) -> @location(0) vec4<f32> {
    // The glossy colour shader cuts on the texture alpha alone.
    if textureSampleBias(image, image_sampler, in.uv, camera.bias.x).a < 0.5 { discard; }
    return motion_vector(in);
}
