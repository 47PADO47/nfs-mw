//! Pixel checks for the actual UI recomposition shader, including additive HUD elements.
use blackbox_gpu_passes::test_support::{Gpu, serial};

#[test]
#[ignore = "requires a Vulkan or DX12 adapter"]
fn separate_ui_preserves_scene_alpha_and_additive_composition() {
    let _serial = serial();
    let gpu = Gpu::new(match super::super::test_support::test_api() {
        crate::Backend::Dx12 => wgpu::Backends::DX12,
        _ => wgpu::Backends::VULKAN,
    });
    let scene = gpu.target(2);
    let raw = gpu.target(2);
    let output = gpu.target(2);
    let ui = gpu.target(2);
    for (texture, pixels) in
        [(&scene, [32, 64, 96, 255].repeat(4)), (&raw, vec![0, 0, 0, 0, 128, 0, 0, 255, 0, 32, 0, 128, 16, 0, 0, 0])]
    {
        gpu.queue.write_texture(
            texture.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(8), rows_per_image: Some(2) },
            texture.size(),
        );
    }
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    super::composition::Composite::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm).encode(
        &gpu.device,
        &mut encoder,
        &scene.create_view(&Default::default()),
        &raw.create_view(&Default::default()),
        &[&output.create_view(&Default::default()), &ui.create_view(&Default::default())],
    );
    let final_pixels = gpu.finish(encoder, &output);
    let encoder = gpu.device.create_command_encoder(&Default::default());
    let ui_pixels = gpu.finish(encoder, &ui);
    assert_eq!(&ui_pixels[..4], &[0, 0, 0, 0], "no HUD coverage outside UI");
    assert_eq!(&final_pixels[..4], &[32, 64, 96, 255], "HUDless background stays unchanged");
    assert_eq!(&ui_pixels[4..8], &[128, 0, 0, 255], "opaque UI excludes the background");
    assert_eq!(&final_pixels[12..16], &[48, 64, 96, 255], "additive UI retains its real-frame blend");
    for pixel in 0..4 {
        let alpha = f32::from(ui_pixels[pixel * 4 + 3]) / 255.0;
        for channel in 0..3 {
            let background = [32.0, 64.0, 96.0][channel];
            let recomposed = f32::from(ui_pixels[pixel * 4 + channel]) + (1.0 - alpha) * background;
            assert!((recomposed - f32::from(final_pixels[pixel * 4 + channel])).abs() <= 1.1);
        }
    }
}

#[test]
#[ignore = "requires a Vulkan or DX12 adapter"]
fn hdr10_preserves_absolute_luminance_gamut_and_separate_ui_alpha() {
    let _serial = serial();
    let gpu = Gpu::new(match super::super::test_support::test_api() {
        crate::Backend::Dx12 => wgpu::Backends::DX12,
        _ => wgpu::Backends::VULKAN,
    });
    let texture = |format| {
        gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("HDR10 composition fixture"),
            size: wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    };
    let scene = texture(wgpu::TextureFormat::Rgba16Float);
    let raw = texture(wgpu::TextureFormat::Rgba16Float);
    let output = texture(wgpu::TextureFormat::Rgb10a2Unorm);
    let ui = texture(wgpu::TextureFormat::Rgba16Float);
    let hudless = texture(wgpu::TextureFormat::Rgb10a2Unorm);
    let mut background = Vec::new();
    let mut foreground = Vec::new();
    for y in 0..4 {
        for nits in [0.0, 80.0, 203.0, 1000.0] {
            background.extend([nits / 80.0, nits / 80.0, nits / 80.0, 1.0]);
            foreground.extend(match y {
                0 => [0.0; 4],
                1 => [203.0 / 80.0, 0.0, 0.0, 1.0], // calibrated red UI white
                2 => [203.0 / 80.0 * 0.25, 0.0, 0.0, 0.25],
                _ => [0.5, 0.0, 0.0, 0.0], // additive overlay
            });
        }
    }
    for (target, pixels) in [(&scene, background), (&raw, foreground)] {
        let bytes: Vec<_> = pixels.into_iter().flat_map(|v| half::f16::from_f32(v).to_le_bytes()).collect();
        gpu.queue.write_texture(
            target.as_image_copy(),
            &bytes,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(32), rows_per_image: Some(4) },
            target.size(),
        );
    }
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    super::composition::Composite::new(&gpu.device, wgpu::TextureFormat::Rgb10a2Unorm).encode(
        &gpu.device,
        &mut encoder,
        &scene.create_view(&Default::default()),
        &raw.create_view(&Default::default()),
        &[
            &output.create_view(&Default::default()),
            &ui.create_view(&Default::default()),
            &hudless.create_view(&Default::default()),
        ],
    );
    gpu.queue.submit([encoder.finish()]);
    let final_bytes = read(&gpu, &output, 4);
    let hudless_bytes = read(&gpu, &hudless, 4);
    let ui_bytes = read(&gpu, &ui, 8);
    let unpack = |bytes: &[u8], pixel: usize, channel: usize| {
        let packed = u32::from_le_bytes(bytes[pixel * 4..][..4].try_into().unwrap());
        ((packed >> (channel * 10)) & 1023) as f64 / 1023.0
    };
    // ST 2084 reference values, independent of the shader implementation.
    for (pixel, expected) in [0.0, 0.485857, 0.580689, 0.751827].into_iter().enumerate() {
        assert!((unpack(&final_bytes, pixel, 0) - expected).abs() < 1.0 / 1023.0);
        assert_eq!(&ui_bytes[pixel * 8..][..8], &[0; 8], "empty HUD has no alpha or RGB");
    }
    for pixel in 0..16 {
        let layer: [f64; 4] = std::array::from_fn(|channel| {
            let offset = pixel * 8 + channel * 2;
            f64::from(half::f16::from_le_bytes(ui_bytes[offset..][..2].try_into().unwrap()).to_f32())
        });
        if pixel / 4 == 2 {
            assert!((layer[3] - 0.25).abs() < 0.001, "UI alpha is not 2-bit");
        }
        for (channel, value) in layer[..3].iter().enumerate() {
            let recomposed = value + (1.0 - layer[3]) * unpack(&hudless_bytes, pixel, channel);
            assert!((recomposed - unpack(&final_bytes, pixel, channel)).abs() < 2.0 / 1023.0);
        }
    }
    // Opaque 203-nit BT.709 red must be converted to the BT.2020 primaries, not merely PQ encoded.
    for (channel, coefficient) in [0.6274039, 0.0690973, 0.0163914].into_iter().enumerate() {
        let p = unpack(&final_bytes, 4, channel).powf(32.0 / 2523.0);
        let nits =
            ((p - 3424.0 / 4096.0).max(0.0) / (2413.0 / 128.0 - 2392.0 / 128.0 * p)).powf(16384.0 / 2610.0) * 10000.0;
        assert!((nits - 203.0 * coefficient).abs() < 1.0);
    }
}

fn read(gpu: &Gpu, texture: &wgpu::Texture, bytes_per_pixel: u32) -> Vec<u8> {
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("HDR fixture readback"),
        size: 256 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(256), rows_per_image: Some(4) },
        },
        texture.size(),
    );
    gpu.queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    gpu.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let mapped = slice.get_mapped_range().unwrap();
    (0..4).flat_map(|y| mapped[y * 256..][..4 * bytes_per_pixel as usize].iter().copied()).collect()
}

#[test]
#[ignore = "requires a Vulkan or DX12 adapter"]
fn additive_hud_over_translucent_panel_keeps_coverage_independent_of_hdr_white() {
    use blackbox_gfx::{UiLayer, UiMesh, UiTextureComposition, UiTextureId, UiTexturePatch, UiVertex};
    let _serial = serial();
    let gpu = Gpu::new(match super::super::test_support::test_api() {
        crate::Backend::Dx12 => wgpu::Backends::DX12,
        _ => wgpu::Backends::VULKAN,
    });
    let mut pass = blackbox_gpu_passes::UiPass::new(&gpu.device, None);
    for (id, rgba) in [(1, [255; 4]), (2, [255, 255, 255, 0])] {
        pass.update_texture(
            &gpu.device,
            &gpu.queue,
            &UiTexturePatch { id: UiTextureId::from_raw(id), offset: None, size: [1, 1], rgba: &rgba },
        )
        .unwrap();
    }
    let quad = |x0: f32, x1: f32, color_rgba, texture| UiMesh {
        texture: UiTextureId::from_raw(texture),
        clip: [0.0, 0.0, 4.0, 3.0],
        vertices: [[x0, 0.0], [x1, 0.0], [x1, 4.0], [x0, 4.0]]
            .into_iter()
            .map(|position| UiVertex { position, uv: [0.5; 2], color_rgba })
            .collect(),
        indices: vec![0, 1, 2, 0, 2, 3],
    };
    let layer = UiLayer {
        compositions: [false, true]
            .map(|additive| UiTextureComposition {
                output: UiTextureId::from_raw(3 + u64::from(additive)),
                picture: UiTextureId::from_raw(1),
                mask: UiTextureId::from_raw(1),
                pivot: [0.5; 2],
                degrees: 33.25,
                window: [0.0, 0.0, 1.0, 1.0],
                additive,
            })
            .into(),
        pixels_per_point: 1.0,
        meshes: vec![
            quad(0.0, 3.0, [0, 0, 0, 64], 3),      // GPU-composed translucent pursuit-panel backing
            quad(0.0, 1.0, [255, 255, 255, 0], 3), // additive vertex colour
            quad(1.0, 2.0, [255; 4], 4),           // GPU-composed additive texture alpha
            quad(2.0, 3.0, [128, 128, 128, 0], 3), // antialiased glyph edge
            quad(3.0, 4.0, [0, 0, 0, 255], 3),     // black opaque UI still has coverage
        ],
    };
    pass.set_layer(layer.clone());
    let texture = |format| {
        gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("additive HUD coverage regression"),
            size: wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    };
    for white in [None, Some(16.0), Some(203.0), Some(500.0)] {
        pass.set_hdr_white(white);
        let format = match white {
            Some(_) => wgpu::TextureFormat::Rgba16Float,
            None => wgpu::TextureFormat::Rgba8Unorm,
        };
        let color = texture(format);
        let reference = texture(format);
        let mask = texture(wgpu::TextureFormat::R16Float);
        let views = [&color, &reference, &mask].map(|t| t.create_view(&Default::default()));
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        for view in &views {
            let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear coverage fixture"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        pass.encode_with_coverage(&gpu.device, &gpu.queue, &mut encoder, &views[0], &views[2], (4, 4));
        pass.encode(&gpu.device, &gpu.queue, &mut encoder, &views[1], format, (4, 4));
        gpu.queue.submit([encoder.finish()]);
        let coverage: Vec<_> =
            read(&gpu, &mask, 2).as_chunks::<2>().0.iter().map(|v| half::f16::from_le_bytes(*v).to_f32()).collect();
        assert_eq!(&coverage[..2], &[1.0, 1.0], "additive glyphs must not inherit panel alpha; white={white:?}");
        let edge = 64.0 / 255.0 + 128.0 / 255.0 * (1.0 - 64.0 / 255.0);
        assert!((coverage[2] - edge).abs() < 0.001, "preserve soft glyph coverage");
        assert_eq!(coverage[3], 1.0, "black opaque UI still covers the scene");
        assert_eq!(&coverage[12..], &[0.0; 4], "clipped/outside HUD remains uncovered");
        let bytes = match white {
            Some(_) => 8,
            None => 4,
        };
        assert_eq!(
            read(&gpu, &color, bytes),
            read(&gpu, &reference, bytes),
            "coverage must not alter real-frame UI blending"
        );
    }
}
