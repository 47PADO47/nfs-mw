// Shared by every post-process effect (concatenated in front of the effect's own shader): the
// bindings of a fullscreen filter and the oversized triangle that covers the screen.

struct Params {
    a: vec4<f32>,
    b: vec4<f32>,
};

@group(0) @binding(0) var src: texture_2d<f32>;
// A second input, for the effects that combine two images (otherwise the same view as `src`).
@group(0) @binding(1) var src2: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;
@group(0) @binding(3) var<uniform> params: Params;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VsOut;
    out.clip = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(p.x, 1.0 - p.y);
    return out;
}
