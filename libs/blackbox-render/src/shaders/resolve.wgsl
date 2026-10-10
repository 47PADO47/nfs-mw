// Resolve: the HDR scene image to the output image. Clamp (identity tonemap), alpha 1.

@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// One oversized triangle covers the screen.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VsOut;
    out.clip = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(p.x, 1.0 - p.y);
    return out;
}

// Same size in and out: copy texels exactly.
@fragment
fn fs_copy(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(scene, vec2<i32>(in.clip.xy), 0);
    return vec4<f32>(clamp(c.rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}

// Different sizes: bilinear filtering (a placeholder until upscalers arrive).
@fragment
fn fs_scaled(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSampleLevel(scene, samp, in.uv, 0.0);
    return vec4<f32>(clamp(c.rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
