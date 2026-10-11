// RH capture images need a vertical flip for standard cube addressing.
// The scene writes gamma-space colour into an unorm face: preserve its bytes.
@fragment fn fs_store(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(src, samp, vec2(in.uv.x, 1.0 - in.uv.y));
}
