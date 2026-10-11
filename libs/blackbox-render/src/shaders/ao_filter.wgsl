// Depth/normal-aware spatial denoising. Never blends across unrelated surfaces.
@group(0) @binding(2) var source_ao: texture_2d<f32>;
@group(0) @binding(3) var filtered_ao: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8)
fn denoise(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= vec2<u32>(params.sizes.zw)) { return; }
    let pixel = vec2<i32>(id.xy);
    let data = textureLoad(source_ao, pixel, 0);
    if data.y == 0.0 {
        textureStore(filtered_ao, id.xy, data);
        return;
    }
    let center = view_at(vec2<f32>(ao_pixel(pixel)), data.y);
    let normal = decode_normal(data.zw);
    var sum = data.x;
    var weight_sum = 1.0;
    for (var y = -2; y <= 2; y++) {
        for (var x = -2; x <= 2; x++) {
            if x == 0 && y == 0 { continue; }
            let neighbor = pixel + vec2<i32>(x, y);
            if any(neighbor < vec2<i32>(0)) || any(neighbor >= vec2<i32>(params.sizes.zw)) { continue; }
            let value = textureLoad(source_ao, neighbor, 0);
            let weight = surface_weight(center, normal, neighbor, value) * exp(-f32(x * x + y * y) * 0.18);
            sum += value.x * weight;
            weight_sum += weight;
        }
    }
    textureStore(filtered_ao, id.xy, vec4<f32>(sum / weight_sum, data.yzw));
}
