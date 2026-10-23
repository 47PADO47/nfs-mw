struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) transform: vec4<f32>,
    @location(2) @interpolate(flat) gain: vec2<f32>,
}
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var displacement: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;
@vertex fn vs_main(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>,
    @location(2) transform: vec4<f32>, @location(3) gain: vec2<f32>) -> Vertex {
    var out: Vertex;
    out.position = vec4(position, 0.0, 1.0);
    out.uv = uv;
    out.transform = transform;
    out.gain = gain;
    return out;
}
@fragment fn fs_main(in: Vertex) -> @location(0) vec4<f32> {
    let offset = textureSample(displacement, samp, in.uv);
    let uv = offset.gb * in.transform.zw + in.transform.xy;
    let color = textureSample(scene, samp, uv).rgb * in.gain.y;
    return vec4(color, clamp(offset.a * in.gain.x, 0.0, 1.0));
}
