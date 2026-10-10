// The 2D UI layer: textured triangles in points, premultiplied-alpha vertex colours.
// Colours and textures are used as stored (gamma space), which is what UI libraries expect
// when the target is a plain UNORM surface.

struct Globals {
    // xy = window size in points
    screen: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VsIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    let p = in.position / globals.screen.xy;
    var out: VsOut;
    out.clip = vec4<f32>(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

// The exact sRGB decode, so decode then the hardware's encode on write is the identity (a renderer whose
// render target view is sRGB compiles this module with `SRGB_TARGET` set; see `world::is_srgb`).
fn linear_from_gamma(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, in.uv) * in.color;
    @if(SRGB_TARGET) {
        return vec4<f32>(linear_from_gamma(c.rgb), c.a);
    } @else {
        return c;
    }
}
