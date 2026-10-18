// HDR10 uses BT.2020 primaries and absolute SMPTE ST 2084 PQ (10,000 nits at 1).
// Inputs retain the renderer's linear BT.709 scRGB convention (1 = 80 nits).
fn hdr10_encode(scrgb: vec3<f32>) -> vec3<f32> {
    let rec2020 = vec3(
        dot(scrgb, vec3(0.6274039, 0.3292830, 0.0433131)),
        dot(scrgb, vec3(0.0690973, 0.9195404, 0.0113623)),
        dot(scrgb, vec3(0.0163914, 0.0880133, 0.8955953)));
    let luminance = clamp(rec2020 * (80.0 / 10000.0), vec3(0.0), vec3(1.0));
    let power = pow(luminance, vec3(2610.0 / 16384.0));
    return pow((vec3(3424.0 / 4096.0) + (2413.0 / 128.0) * power) /
        (vec3(1.0) + (2392.0 / 128.0) * power), vec3(2523.0 / 32.0));
}

struct HdrComposed {
    @location(0) final_color: vec4<f32>,
    @location(1) ui: vec4<f32>,
    @location(2) hudless: vec4<f32>,
}

@fragment fn fs_hdr10(@builtin(position) pixel: vec4<f32>) -> HdrComposed {
    let position = vec2<i32>(pixel.xy);
    let background = textureLoad(scene, position, 0).rgb;
    let layer = textureLoad(raw_ui, position, 0);
    let additive = layer.a == 0.0 && any(layer.rgb != vec3(0.0));
    let alpha = select(layer.a, 1.0, additive);
    // Blend in linear light before the nonlinear PQ encoding, including UI white calibration.
    let final_pq = hdr10_encode(layer.rgb + (1.0 - layer.a) * background);
    let background_pq = hdr10_encode(background);
    var result: HdrComposed;
    result.final_color = vec4(final_pq, 1.0);
    result.hudless = vec4(background_pq, 1.0);
    // SDK recomposition operates in the tagged output encoding. Compensate the UI term so
    // final_pq = ui.rgb + (1 - ui.a) * background_pq exactly, with no UI alpha quantized to 2 bits.
    result.ui = vec4(final_pq - (1.0 - alpha) * background_pq, alpha);
    return result;
}
