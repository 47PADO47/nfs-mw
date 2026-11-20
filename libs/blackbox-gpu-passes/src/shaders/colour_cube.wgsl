// Colour grading through 3D look-up tables (`ColourCubePass`, docs/specs/colour-grading.md): the finished frame's colour
// picks a position in a cube (red along x, green along y, blue along z, the first entry at 0 and the last at 1) and the
// output is the linearly filtered cube there, optionally blended with a second cube, then moved `strength` of the way from
// the frame's own colour. A value above 1 (a scene that is not tone mapped) keeps its excess: only the part inside 0 to 1
// is graded.
//
// params.a = (strength, mix of the second cube, 1 when there is a second cube, unused)

struct Params {
    a: vec4<f32>,
};

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var cube_a: texture_3d<f32>;
@group(0) @binding(2) var cube_b: texture_3d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var<uniform> params: Params;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VsOut;
    out.clip = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

// The entry centres are at (i + 0.5) / n, so the first entry is hit at 0 and the last at 1.
fn lookup_a(c: vec3<f32>) -> vec3<f32> {
    let n = vec3<f32>(textureDimensions(cube_a));
    return textureSampleLevel(cube_a, samp, (c * (n - vec3<f32>(1.0)) + vec3<f32>(0.5)) / n, 0.0).rgb;
}

fn lookup_b(c: vec3<f32>) -> vec3<f32> {
    let n = vec3<f32>(textureDimensions(cube_b));
    return textureSampleLevel(cube_b, samp, (c * (n - vec3<f32>(1.0)) + vec3<f32>(0.5)) / n, 0.0).rgb;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let original = textureLoad(src, vec2<i32>(in.clip.xy), 0);
    let inside = clamp(original.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    var graded = lookup_a(inside);
    if params.a.z > 0.5 {
        graded = mix(graded, lookup_b(inside), params.a.y);
    }
    let rgb = inside + (graded - inside) * params.a.x + (original.rgb - inside);
    return vec4<f32>(rgb, original.a);
}
