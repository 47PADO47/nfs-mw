// Snapshot matching scenery depth without requiring native interop copy-source flags.
@group(0) @binding(0) var scenery_depth: texture_depth_2d;
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(positions[index],0.0,1.0);
}
@fragment fn fs_main(@builtin(position) pixel: vec4<f32>) -> @builtin(frag_depth) f32 {
    return textureLoad(scenery_depth,vec2<i32>(pixel.xy),0);
}
