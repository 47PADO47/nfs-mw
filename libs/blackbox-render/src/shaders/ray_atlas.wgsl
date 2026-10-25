@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var linear_sampler: sampler;
struct Out { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) i: u32) -> Out {
    let uv = vec2(f32((i<<1u)&2u),f32(i&2u));
    return Out(vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),0.0,1.0),uv);
}
@fragment fn fs(in: Out) -> @location(0) vec4<f32> { return textureSample(source,linear_sampler,in.uv); }
