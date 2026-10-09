//! GPU checks of the FSR 1 passes on a real backend, called the way a foreign render loop would: plain
//! textures in, a command encoder of the caller's, no post chain around them.

use crate::test_support::{Gpu, serial};
use crate::{FSR1_EASU, FSR1_RCAS, Fsr1Io, Fsr1Pass, Fsr1Stage, RcasScale, fsr1_passes};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DARK: u8 = 64;
const LIGHT: u8 = 191;

#[test]
#[ignore = "needs a Vulkan GPU"]
fn fsr1_vulkan() {
    check(wgpu::Backends::VULKAN);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "needs a Direct3D 12 GPU"]
fn fsr1_dx12() {
    check(wgpu::Backends::DX12);
}

/// A `size` x `size` image with a vertical edge: dark on the left half, light on the right.
fn edge_image(gpu: &Gpu, size: u32) -> wgpu::Texture {
    let texture = gpu.target(size);
    let pixels: Vec<u8> = (0..size * size)
        .flat_map(|i| {
            let v = if i % size < size / 2 { DARK } else { LIGHT };
            [v, v, v, 255]
        })
        .collect();
    gpu.queue.write_texture(
        texture.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(size * 4), rows_per_image: Some(size) },
        texture.size(),
    );
    texture
}

fn view(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// Run `passes` in order over an 8x8 edge image, each into a fresh 16x16 image, and return the last
/// one's rows of the red channel.
fn upscale(gpu: &Gpu, passes: &mut [Fsr1Pass]) -> Vec<Vec<u8>> {
    let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let mut current = edge_image(gpu, 8);
    for pass in passes {
        let next = gpu.target(16);
        let io = Fsr1Io { input: &view(&current), output: &view(&next), output_format: FORMAT, output_size: (16, 16) };
        pass.encode(&gpu.device, &gpu.queue, &mut encoder, &io);
        current = next;
    }
    let pixels = gpu.finish(encoder, &current);
    pixels.chunks(16 * 4).map(|row| row.chunks(4).map(|p| p[0]).collect()).collect()
}

fn check(backend: wgpu::Backends) {
    let _lock = serial();
    let gpu = Gpu::new(backend);
    let rcas = RcasScale::new();

    // EASU alone: flat areas stay flat, the edge stays monotonic and never overshoots.
    let mut passes = fsr1_passes(&gpu.device, &rcas, false);
    assert_eq!(passes.iter().map(Fsr1Pass::name).collect::<Vec<_>>(), [FSR1_EASU]);
    let rows = upscale(&gpu, &mut passes);
    let first = &rows[0];
    assert!(rows.iter().all(|r| r == first), "a vertical edge looks the same on every row: {rows:?}");
    assert!(first[0].abs_diff(DARK) <= 2 && first[15].abs_diff(LIGHT) <= 2, "{first:?}");
    assert!(first.windows(2).all(|w| w[0] <= w[1]), "no ringing without RCAS: {first:?}");
    assert!(first.iter().all(|&v| (DARK - 1..=LIGHT + 1).contains(&v)), "{first:?}");
    assert!(first[7] < first[8], "the edge is across the middle: {first:?}");

    // With RCAS the edge is sharper: it overshoots the two levels, and the flat areas are untouched.
    rcas.set_stops(0.0);
    let mut passes = fsr1_passes(&gpu.device, &rcas, true);
    assert_eq!(passes.iter().map(Fsr1Pass::stage).collect::<Vec<_>>(), [Fsr1Stage::Easu, Fsr1Stage::Rcas]);
    assert_eq!(passes[1].name(), FSR1_RCAS);
    let sharp = &upscale(&gpu, &mut passes)[0];
    assert!(sharp[0].abs_diff(DARK) <= 2 && sharp[15].abs_diff(LIGHT) <= 2, "{sharp:?}");
    assert!(sharp[1..7].iter().any(|&v| v < DARK) || sharp[9..15].iter().any(|&v| v > LIGHT), "{sharp:?}");
}
