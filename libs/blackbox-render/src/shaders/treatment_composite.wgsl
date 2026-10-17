// Independent implementation from docs/specs/visual-treatment.md.
struct Parameters {
    tint: vec4<f32>,
    luminance: vec4<f32>,
    strengths: vec4<f32>, // source-colour coefficient, black, colour, treatment blend
    black_curve: vec4<f32>,
    colour_curve: vec4<f32>,
}
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var luma: texture_2d<f32>;
@group(0) @binding(2) var gain: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var<uniform> p: Parameters;

struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> VsOut {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: VsOut;
    out.pos = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    out.uv = uv;
    return out;
}
fn curve(c: vec4<f32>, x: f32) -> f32 {
    let t = clamp(x, 0.0, 1.0);
    let u = 1.0 - t;
    return dot(c, vec4<f32>(u*u*u, 3.0*u*u*t, 3.0*u*t*t, t*t*t));
}
@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let raw = textureSampleLevel(scene, samp, in.uv, 0.0).rgb;
    var c = clamp(raw, vec3<f32>(0.0), vec3<f32>(1.0));
    if p.tint.w > 0.5 { c = max(raw,vec3(0.0)); }
    var treated = c;
    if p.strengths.w > 0.0 {
        let local_luma = textureSampleLevel(luma, samp, in.uv, 0.0).r;
        // The historical Desaturation field weights source colour, not grayscale.
        let base = mix(vec3<f32>(dot(c, p.luminance.xyz)), c, p.strengths.x);
        let black = mix(1.0, curve(p.black_curve, local_luma), p.strengths.y);
        let colour = curve(p.colour_curve, local_luma) * p.strengths.z;
        let screen_gain = textureSampleLevel(gain, samp, vec2<f32>(in.uv.x, 1.0-in.uv.y), 0.0).b;
        treated = mix(c, (base * black + c * p.tint.xyz * colour) * screen_gain, p.strengths.w);
    }
    if p.tint.w > 0.5 { return vec4(max(treated,vec3(0.0)),1.0); }
    return vec4<f32>(clamp(treated, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
