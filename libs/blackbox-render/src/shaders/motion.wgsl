// Motion vectors for the temporal upscaler: where each opaque surface point was in the previous frame, minus
// where it is now, in normalised device coordinates (y up). Drawn after the scene, against its finished depth
// buffer, so each pixel gets the vector of the surface the colour pass left visible.
//
// Two draws share the target. fs_fill covers the screen first: it rebuilds the world position of every pixel from
// the depth buffer and gives it the camera's motion alone, which is right for everything that stood still. The
// instances whose transform changed since the last frame are then drawn over it (vs_main with the fs_* stages)
// with their own previous matrix.

struct Camera {
    // World to clip with this frame's sub-pixel jitter: the position the colour pass drew at.
    jittered: mat4x4<f32>,
    // World to clip without jitter, this frame and the last: the vectors must not contain the jitter.
    current: mat4x4<f32>,
    previous: mat4x4<f32>,
    // Inverse of `jittered`, to rebuild the world position of a pixel from the depth buffer (fs_fill).
    inverse: mat4x4<f32>,
    // x = texture LOD bias of the colour pass, so alpha-tested edges are cut the same way.
    bias: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
// The scene's finished depth buffer (reverse-Z, 0 where nothing was drawn); fs_fill only.
@group(0) @binding(1) var depth_texture: texture_depth_2d;
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
    @location(12) shadow_flags: vec4<f32>,
};
fn instance_model(i: InstanceIn) -> mat4x4<f32> { return mat4x4<f32>(i.model_0, i.model_1, i.model_2, i.model_3); }
fn instance_receives_shadow(i: InstanceIn) -> u32 { return u32(i.shadow_flags.x); }
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
fn motion_between(current: vec4<f32>, previous: vec4<f32>) -> vec4<f32> {
    if previous.w <= 1e-6 || current.w <= 1e-6 {
        return vec4<f32>(0.0);
    }
    let motion = previous.xy / previous.w - current.xy / current.w;
    // RG motion targets discard Z/W. AO's RGBA target also needs the old object-space
    // point's view depth, rather than predicting its depth from camera motion alone.
    return vec4<f32>(motion, previous.w, 1.0);
}

fn motion_vector(in: Out) -> vec4<f32> {
    return motion_between(in.current, in.previous);
}

// One oversized triangle over the whole target.
@vertex
fn vs_fill(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

// The camera's motion of the surface the depth buffer holds at this pixel. The world position comes from the
// jittered matrix the depth was drawn with; both frames then project it without jitter, as the object draws do.
@fragment
fn fs_fill(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let texel = vec2<i32>(frag.xy);
    let depth = textureLoad(depth_texture, texel, 0);
    let size = vec2<f32>(textureDimensions(depth_texture));
    let ndc = vec2<f32>(frag.x / size.x * 2.0 - 1.0, 1.0 - frag.y / size.y * 2.0);
    let h = camera.inverse * vec4<f32>(ndc, depth, 1.0);
    // Reverse-Z depth zero is a direction at infinity. Project it without translation
    // so camera rotation moves the sky, while AO still sees no surface coverage.
    if depth <= 0.0 {
        let direction = vec4<f32>(h.xyz, 0.0);
        let sky_motion = motion_between(camera.current * direction, camera.previous * direction);
        return vec4<f32>(sky_motion.xy, 0.0, 0.0);
    }
    if abs(h.w) < 1e-20 {
        return vec4<f32>(0.0);
    }
    let world = vec4<f32>(h.xyz / h.w, 1.0);
    return motion_between(camera.current * world, camera.previous * world);
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
