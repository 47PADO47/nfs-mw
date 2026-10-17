//! Float readback against independent pinned HLSL vectors; no game assets.
use super::{display, display_vectors as reference};
use crate::gpu::test_support;
use blackbox_gpu_passes::{Blend, Draw, Filter, Params};

#[test]
fn display_shader_validates_and_has_explicit_float_output() {
    let source = display::source();
    let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
        .validate(&module)
        .unwrap();
    assert_eq!(module.entry_points.len(), 2);
}

#[test]
#[ignore = "requires a real GPU"]
fn psychov_float_reference_vectors() {
    let _serial = test_support::serial();
    let r = test_support::headless((32, 1)).expect("HDR reference adapter");
    let (device, queue) = (&r.device, &r.queue);
    let width = reference::INPUT.len() as u32;
    let texture = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("display reference"),
            size: wgpu::Extent3d { width, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let input =
        texture(wgpu::TextureFormat::Rgba16Float, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
    let output = texture(
        wgpu::TextureFormat::Rgba32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let values: Vec<u16> = reference::INPUT.iter().flatten().map(|&v| half::f16::from_f32(v).to_bits()).collect();
    queue.write_texture(
        input.as_image_copy(),
        bytemuck::cast_slice(&values),
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 8), rows_per_image: Some(1) },
        input.size(),
    );
    let src = input.create_view(&Default::default());
    let target = output.create_view(&Default::default());
    let mut filter = Filter::new(device, "PsychoV numerical reference", &display::source());
    for (params, expected) in reference::CASES {
        filter.set_params(queue, &Params { a: *params, b: [0.0, width as f32, 1.0, 0.0] });
        let mut encoder = device.create_command_encoder(&Default::default());
        filter.draw(
            device,
            &mut encoder,
            &Draw {
                entry: "fs_main",
                src: &src,
                src2: &src,
                target: &target,
                format: output.format(),
                blend: Blend::Replace,
            },
        );
        queue.submit([encoder.finish()]);
        let bytes = r.read_raw_texture(&output, (width, 1), 16).unwrap();
        let actual: &[f32] = bytemuck::cast_slice(&bytes);
        for (index, (&a, &b)) in actual.iter().zip(expected.iter().flatten()).enumerate() {
            let tolerance = 0.003 * b.abs().max(0.05);
            assert!(
                a.is_finite() && (a - b).abs() <= tolerance,
                "params={params:?}, channel={index}, got={a}, reference={b}"
            );
        }
        if params[2] > 0.5 {
            assert!(actual.iter().any(|&v| v > 1.0), "HDR highlights must survive above scRGB 1.0");
        }
    }
}
