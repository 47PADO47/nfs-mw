//! Capture storage preserves exact orientation/bytes and removes finite scenery continuously.
use super::{reflection_store::Store, test_support};

#[test]
#[ignore = "needs a GPU"]
fn cube_store_preserves_pixels_and_sky_and_fades_the_outer_boundary() {
    let _serial = test_support::serial();
    let r = test_support::headless((2, 2)).expect("GPU adapter");
    let color = r.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("orientation fixture"),
        size: wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let pixels = [20, 40, 80, 255, 40, 70, 100, 255, 120, 160, 200, 255, 180, 200, 240, 255];
    r.queue.write_texture(
        color.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(8), rows_per_image: Some(2) },
        wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 1 },
    );
    let depths = r.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("boundary depth fixture"),
        size: wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 6 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depths.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let mut store = Store::new(&r.device, &color.create_view(&Default::default()), &depth_view, None);
    let output = r.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("boundary result"),
        size: wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = output.create_view(&Default::default());
    let flipped = [120, 160, 200, 255, 180, 200, 240, 255, 20, 40, 80, 255, 40, 70, 100, 255];
    for (radius, far, fade) in [
        (20.0, None, 0.0),
        (0.0, Some(10.0), 0.0),
        (8.0, Some(10.0), 0.0),
        (9.25, Some(10.0), 0.5),
        (10.1, Some(10.0), 1.0),
        (20.0, None, 0.0),
    ] {
        let depth = match radius > 0.0 {
            true => 0.25 * 1.5_f32.sqrt() / radius,
            false => 0.0,
        };
        let mut encoder = r.device.create_command_encoder(&Default::default());
        for face in 0..6 {
            let face_view = depths.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: face,
                array_layer_count: Some(1),
                ..Default::default()
            });
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &face_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(depth), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
        }
        store.configure(&r.queue, far, [0.2, 0.3, 0.4]);
        store.draw(&mut encoder, &view, 0);
        r.queue.submit([encoder.finish()]);
        let actual = r.read_raw_texture(&output, (2, 2), 4).unwrap();
        for (i, byte) in actual.iter().enumerate() {
            let channel = i % 4;
            let fog = [51.0, 76.5, 102.0, 255.0][channel];
            let expected = (f32::from(flipped[i]) * (1.0 - fade) + fog * fade).round() as u8;
            assert!(byte.abs_diff(expected) <= 1, "radius {radius}, channel {i}: {byte} vs {expected}");
        }
        if fade == 0.0 {
            assert_eq!(actual, flipped);
        }
    }
}
