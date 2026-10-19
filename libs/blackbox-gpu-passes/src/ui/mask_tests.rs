//! Compare persistent GPU mask pixels against the previous CPU sampling/rounding contract.
use super::*;
use crate::test_support::{Gpu, serial};
use blackbox_gfx::{UiLayer, UiTextureId, UiTexturePatch};

#[test]
#[ignore = "needs a Vulkan GPU"]
fn ui_masks_vulkan() {
    check(wgpu::Backends::VULKAN);
}

#[cfg(windows)]
#[test]
#[ignore = "needs a Direct3D 12 GPU"]
fn ui_masks_dx12() {
    check(wgpu::Backends::DX12);
}

fn reference(picture: &[u8], mask: &[u8], size: u32, c: UiTextureComposition) -> Vec<u8> {
    let (mw, mh) = (19.0, 11.0);
    let (sin, cos) = c.degrees.to_radians().sin_cos();
    let (cx, cy) = (mw * 0.5, mh * 0.5);
    let (px, py) = (mw * c.pivot[0] - cx, mh * c.pivot[1] - cy);
    let mut result = picture.to_vec();
    for y in 0..size {
        for x in 0..size {
            let u = c.window[0] + (x as f32 + 0.5) / size as f32 * (c.window[2] - c.window[0]);
            let v = c.window[1] + (y as f32 + 0.5) / size as f32 * (c.window[3] - c.window[1]);
            let (sx, sy) = (u * mw - cx - px, v * mh - cy - py);
            let (qx, qy) = (sx * cos + sy * sin + px + cx, sy * cos - sx * sin + py + cy);
            let mut coverage = 0.0;
            if (0.0..=mw).contains(&qx) && (0.0..=mh).contains(&qy) {
                let (fx, fy) = (qx - 0.5, qy - 0.5);
                let (ix, iy) = (fx.floor() as i32, fy.floor() as i32);
                let (tx, ty) = (fx - fx.floor(), fy - fy.floor());
                let sample =
                    |x: i32, y: i32| f32::from(mask[((y.clamp(0, 10) * 19 + x.clamp(0, 18)) * 4 + 3) as usize]) / 255.0;
                let top = sample(ix, iy) * (1.0 - tx) + sample(ix + 1, iy) * tx;
                let bottom = sample(ix, iy + 1) * (1.0 - tx) + sample(ix + 1, iy + 1) * tx;
                coverage = top * (1.0 - ty) + bottom * ty;
            }
            let pixel = &mut result[((y * size + x) * 4) as usize..][..4];
            let alpha = (f32::from(pixel[3]) * coverage + 0.5) as u8;
            for channel in &mut pixel[..3] {
                *channel = ((u32::from(*channel) * u32::from(alpha) + 127) / 255) as u8;
            }
            pixel[3] = match c.additive {
                true => 0,
                false => alpha,
            };
        }
    }
    result
}

fn check(backend: wgpu::Backends) {
    let _lock = serial();
    let gpu = Gpu::new(backend);
    let mut ui = UiPass::new(&gpu.device, None);
    let [picture_id, mask_id, output] = [1, 2, 3].map(UiTextureId::from_raw);
    let mask: Vec<u8> = (0..19 * 11).flat_map(|i| [255, 128, 64, ((i * 37) % 256) as u8]).collect();
    ui.update_texture(
        &gpu.device,
        &gpu.queue,
        &UiTexturePatch { id: mask_id, offset: None, size: [19, 11], rgba: &mask },
    )
    .unwrap();
    let target = gpu.target(1);
    let view = target.create_view(&Default::default());
    for size in [13, 32, 64] {
        let picture: Vec<u8> = (0..size * size).flat_map(|i| [240, 120, 60, ((i * 53) % 256) as u8]).collect();
        ui.update_texture(
            &gpu.device,
            &gpu.queue,
            &UiTexturePatch { id: picture_id, offset: None, size: [size, size], rgba: &picture },
        )
        .unwrap();
        for degrees in [0.0, 33.25, -90.0, 180.0] {
            for (pivot, window) in [
                ([0.5, 0.5], [0.0, 0.0, 1.0, 1.0]),
                ([0.13, 0.91], [-0.75, -0.25, 0.25, 0.75]),
                ([0.5, 0.5], [0.83, 1.31, -0.27, -0.17]),
            ] {
                for additive in [false, true] {
                    let command = UiTextureComposition {
                        output,
                        picture: picture_id,
                        mask: mask_id,
                        pivot,
                        degrees,
                        window,
                        additive,
                    };
                    let mut encoder = gpu.device.create_command_encoder(&Default::default());
                    ui.set_layer(UiLayer { compositions: vec![command], ..Default::default() });
                    ui.encode(&gpu.device, &gpu.queue, &mut encoder, &view, super::super::TEXTURE_FORMAT, (1, 1));
                    let pixels = gpu.finish(encoder, &ui.textures[&output.raw()].texture);
                    let expected = reference(&picture, &mask, size, command);
                    let error = pixels.iter().zip(&expected).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
                    assert!(error <= 1, "{backend:?}: {size}, {command:?}, max error {error}");
                    // No new command: re-encoding must preserve the composed texture exactly.
                    ui.set_layer(UiLayer::default());
                    let mut encoder = gpu.device.create_command_encoder(&Default::default());
                    ui.encode(&gpu.device, &gpu.queue, &mut encoder, &view, super::super::TEXTURE_FORMAT, (1, 1));
                    assert_eq!(pixels, gpu.finish(encoder, &ui.textures[&output.raw()].texture));
                }
            }
        }
    }
}
