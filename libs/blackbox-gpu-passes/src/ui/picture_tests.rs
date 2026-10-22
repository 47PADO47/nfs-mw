//! The picture pass against the integer tint-and-clip it replaces on the CPU: the two must agree to the bit.
use super::*;
use crate::test_support::{Gpu, serial};
use blackbox_gfx::UiTexturePatch;

#[test]
#[ignore = "needs a Vulkan GPU"]
fn ui_pictures_vulkan() {
    check(wgpu::Backends::VULKAN);
}

#[cfg(windows)]
#[test]
#[ignore = "needs a Direct3D 12 GPU"]
fn ui_pictures_dx12() {
    check(wgpu::Backends::DX12);
}

/// Tint, then coverage, in integers, as the mirror's CPU path does it.
fn reference(picture: &[u8], mask: &[u8], tint: [u8; 3]) -> Vec<u8> {
    let mut result = Vec::with_capacity(picture.len());
    for (pixel, mask) in picture.as_chunks::<4>().0.iter().zip(mask.as_chunks::<4>().0) {
        let cover = u16::from(mask[3]);
        for (channel, factor) in pixel[..3].iter().zip(tint) {
            let tinted = (u16::from(*channel) * u16::from(factor) + 127) / 255;
            result.push(((tinted * cover + 127) / 255) as u8);
        }
        result.push(mask[3]);
    }
    result
}

fn check(backend: wgpu::Backends) {
    let _lock = serial();
    let gpu = Gpu::new(backend);
    let mut ui = UiPass::new(&gpu.device, None);
    let [mask_id, output] = [2, 3].map(UiTextureId::from_raw);
    for size in [1u32, 13, 32, 64] {
        let texels = (size * size) as usize;
        let picture: Vec<u8> = (0..texels * 4).map(|i| (i * 37 % 256) as u8).collect();
        let mask: Vec<u8> = (0..texels).flat_map(|i| [9, 9, 9, ((i * 53) % 256) as u8]).collect();
        let source = gpu.target(size);
        gpu.queue.write_texture(
            source.as_image_copy(),
            &picture,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(size * 4), rows_per_image: Some(size) },
            source.size(),
        );
        ui.update_texture(
            &gpu.device,
            &gpu.queue,
            &UiTexturePatch { id: mask_id, offset: None, size: [size, size], rgba: &mask },
        )
        .unwrap();
        let view = source.create_view(&Default::default());
        for tint in [[255, 255, 255], [150, 150, 150], [37, 200, 91]] {
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            ui.compose_picture(&gpu.device, &mut encoder, &view, (size, size), mask_id, tint, output).unwrap();
            let pixels = gpu.finish(encoder, &ui.textures[&output.raw()].texture);
            assert_eq!(pixels, reference(&picture, &mask, tint), "{backend:?}: {size}x{size}, tint {tint:?}");
        }
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        let wrong = (size + 1, size);
        assert!(ui.compose_picture(&gpu.device, &mut encoder, &view, wrong, mask_id, [255; 3], output).is_err());
    }
}
