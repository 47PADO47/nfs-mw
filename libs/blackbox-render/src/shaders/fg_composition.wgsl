// Preserve the original blend and provide a UI buffer satisfying Streamline's composition contract.
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var raw_ui: texture_2d<f32>;

@vertex fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

struct Composed {
    @location(0) final_color: vec4<f32>,
    @location(1) ui: vec4<f32>,
}

@fragment fn fs_main(@builtin(position) pixel: vec4<f32>) -> Composed {
    let position = vec2<i32>(pixel.xy);
    let background = textureLoad(scene, position, 0).rgb;
    let layer = textureLoad(raw_ui, position, 0);
    // Additive UI uses zero alpha and nonzero RGB. Give it coverage, compensating its RGB
    // with the background so Final = UI.rgb + (1 - UI.a) * Hudless remains unchanged.
    let additive = layer.a == 0.0 && any(layer.rgb != vec3(0.0));
    let alpha = select(layer.a, 1.0, additive);
    var result: Composed;
    result.final_color = vec4(layer.rgb + (1.0 - layer.a) * background, 1.0);
    result.ui = vec4(layer.rgb + (alpha - layer.a) * background, alpha);
    return result;
}
