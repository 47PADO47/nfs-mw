//! Checks of the fullscreen filter helper: the shared shader half validates, and a small effect built on it
//! runs on a real backend (two inputs, parameters, replace and add blending).

use crate::test_support::{Gpu, serial};
use crate::{Blend, Draw, Filter, POST_COMMON_WGSL, Params, filter_source};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// `src * a.x + src2 * b.x`
const BLEND_WGSL: &str = "
@fragment
fn fs_mix(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(src, samp, in.uv) * params.a.x + textureSample(src2, samp, in.uv) * params.b.x;
}
";

#[test]
fn the_shared_half_and_an_effect_validate_together() {
    let source = filter_source(BLEND_WGSL);
    assert!(source.starts_with(POST_COMMON_WGSL), "the common half comes first");
    let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::default())
        .validate(&module)
        .expect("valid");
    let entries: Vec<_> = module.entry_points.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(entries, ["vs_main", "fs_mix"]);
}

#[test]
fn the_parameter_block_is_two_vec4() {
    assert_eq!(std::mem::size_of::<Params>(), 32);
    let bytes = bytemuck::bytes_of(&Params { a: [1.0, 2.0, 3.0, 4.0], b: [5.0, 6.0, 7.0, 8.0] });
    let floats: &[f32] = bytemuck::cast_slice(bytes);
    assert_eq!(floats, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
}

/// A 1x1 image of one grey level.
fn grey(gpu: &Gpu, level: u8) -> wgpu::Texture {
    let texture = gpu.target(1);
    gpu.queue.write_texture(
        texture.as_image_copy(),
        &[level, level, level, 255],
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
        texture.size(),
    );
    texture
}

fn view(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn a_filter_mixes_two_inputs_and_adds_into_the_target() {
    let _lock = serial();
    let gpu = Gpu::new(wgpu::Backends::VULKAN);
    let mut filter = Filter::new(&gpu.device, "test mix", &filter_source(BLEND_WGSL));
    let (a, b, out) = (grey(&gpu, 100), grey(&gpu, 50), gpu.target(1));
    let (a_view, b_view, out_view) = (view(&a), view(&b), view(&out));
    filter.set_params(&gpu.queue, &Params { a: [0.5, 0.0, 0.0, 0.0], b: [0.5, 0.0, 0.0, 0.0] });
    let draw = |blend| Draw { entry: "fs_mix", src: &a_view, src2: &b_view, target: &out_view, format: FORMAT, blend };

    let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    filter.draw(&gpu.device, &mut encoder, &draw(Blend::Replace));
    let replaced = gpu.finish(encoder, &out);
    assert!(replaced[0].abs_diff(75) <= 1, "100 * 0.5 + 50 * 0.5: {replaced:?}");

    // Add keeps what the target holds: 75 + 75 = 150.
    let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    filter.draw(&gpu.device, &mut encoder, &draw(Blend::Add));
    let added = gpu.finish(encoder, &out);
    assert!(added[0].abs_diff(150) <= 2, "{added:?}");
}
