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

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv) * in.color;
}

// The exact sRGB decode, so decode then the hardware's encode on write is the identity; see
// `world::is_srgb`.
fn linear_from_gamma(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_main_srgb(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, in.uv) * in.color;
    return vec4<f32>(linear_from_gamma(c.rgb), c.a);
}

// Decode straight RGB, then reapply alpha for linear premultiplied compositing.
@fragment fn fs_main_hdr(in: VsOut) -> @location(0) vec4<f32> {
    return hdr_color(textureSample(tex, samp, in.uv) * in.color);
}

fn hdr_color(c: vec4<f32>) -> vec4<f32> {
    // Zero alpha with nonzero RGB is the UI layer's additive blend encoding,
    // used by authored map tiles, fonts and egui glows. Zero coverage is black.
    if c.a <= 0.0 {
        return vec4(linear_from_gamma(max(c.rgb, vec3(0.0))) * globals.screen.z, 0.0);
    }
    let straight = max(c.rgb / c.a, vec3(0.0));
    return vec4(linear_from_gamma(straight) * c.a * globals.screen.z, c.a);
}

// Track geometry coverage before blending: an additive glyph over a translucent panel
// must not inherit just the panel's alpha. RGB is still premultiplied in gamma space.
struct CoveredUi {
    @location(0) color: vec4<f32>,
    @location(1) coverage: vec4<f32>,
}
fn covered(c: vec4<f32>, color: vec4<f32>) -> CoveredUi {
    let additive = c.a == 0.0;
    let opacity = clamp(select(c.a, max(c.r, max(c.g, c.b)), additive), 0.0, 1.0);
    // Preserve very soft edges while giving visible translucent HUD layers strong protection
    // from scene interpolation. This changes only the FG mask, not real-frame UI blending.
    let alpha = select(opacity, max(opacity, 0.9), opacity > 0.05);
    var out: CoveredUi;
    out.color = color;
    out.coverage = vec4(alpha);
    return out;
}
@fragment fn fs_coverage(in: VsOut) -> CoveredUi {
    let c = textureSample(tex, samp, in.uv) * in.color;
    return covered(c, c);
}
@fragment fn fs_coverage_srgb(in: VsOut) -> CoveredUi {
    let c = textureSample(tex, samp, in.uv) * in.color;
    return covered(c, vec4(linear_from_gamma(c.rgb), c.a));
}
@fragment fn fs_coverage_hdr(in: VsOut) -> CoveredUi {
    let c = textureSample(tex, samp, in.uv) * in.color;
    return covered(c, hdr_color(c));
}
