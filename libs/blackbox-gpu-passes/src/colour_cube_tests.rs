//! The WGSL validates without a GPU; the GPU tests (`--ignored`) compare the pass with the CPU reference
//! (`ColourGrade::apply`) and check the identity cube.

use std::sync::Arc;

use blackbox_gfx::{ColourCube, ColourGrade};

use super::*;
use crate::test_support::{Gpu, serial};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[test]
fn the_shader_parses_and_validates() {
    let module = naga::front::wgsl::parse_str(COLOUR_CUBE_WGSL)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(COLOUR_CUBE_WGSL)));
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::default())
        .validate(&module)
        .expect("valid");
    let entries: Vec<_> = module.entry_points.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(entries, ["vs_main", "fs_main"]);
}

#[test]
fn the_parameter_block_is_one_vec4() {
    assert_eq!(std::mem::size_of::<Block>(), 16);
}

/// A cube that is not a ramp: it swaps and bends the channels.
fn bent(size: u32) -> ColourCube {
    let n = size as usize;
    let mut rgb = Vec::new();
    for z in 0..n {
        for y in 0..n {
            for x in 0..n {
                let (r, g, b) = (x as f32 / (n - 1) as f32, y as f32 / (n - 1) as f32, z as f32 / (n - 1) as f32);
                rgb.extend([g, b * b, 1.0 - r].map(|v| (v * 255.0).round() as u8));
            }
        }
    }
    ColourCube::new(size, rgb).unwrap()
}

/// A 16 x 16 image with red from the column, green from the row and the given blue.
fn chart(gpu: &Gpu, blue: u8) -> wgpu::Texture {
    let texture = gpu.target(16);
    let bytes: Vec<u8> =
        (0..16u32).flat_map(|y| (0..16u32).flat_map(move |x| [(x * 17) as u8, (y * 17) as u8, blue, 255])).collect();
    gpu.queue.write_texture(
        texture.as_image_copy(),
        &bytes,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(64), rows_per_image: Some(16) },
        texture.size(),
    );
    texture
}

/// Run the pass over the chart with `grade` and return the 16 x 16 RGBA bytes.
fn run(gpu: &Gpu, pass: &mut ColourCubePass, blue: u8, grade: Option<&ColourGrade>) -> Vec<u8> {
    let (input, output) = (chart(gpu, blue), gpu.target(16));
    pass.set(&gpu.device, &gpu.queue, grade);
    let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    pass.encode(
        &gpu.device,
        &gpu.queue,
        &mut encoder,
        &ColourCubeIo {
            input: &input.create_view(&Default::default()),
            output: &output.create_view(&Default::default()),
            output_format: FORMAT,
        },
    );
    gpu.finish(encoder, &output)
}

fn assert_matches_reference(pixels: &[u8], blue: u8, grade: &ColourGrade, tolerance: u8) {
    for y in 0..16usize {
        for x in 0..16usize {
            let want = grade.apply([x as f32 * 17.0 / 255.0, y as f32 * 17.0 / 255.0, f32::from(blue) / 255.0]);
            let got = &pixels[(y * 16 + x) * 4..][..3];
            for channel in 0..3 {
                let expected = (want[channel].clamp(0.0, 1.0) * 255.0).round() as u8;
                assert!(
                    got[channel].abs_diff(expected) <= tolerance,
                    "({x},{y},{blue}) channel {channel}: {got:?} vs {want:?}"
                );
            }
        }
    }
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn the_identity_cube_leaves_the_colours_unchanged_to_a_255th() {
    let _lock = serial();
    let gpu = Gpu::new(wgpu::Backends::VULKAN);
    let mut pass = ColourCubePass::new(&gpu.device);
    for size in [16, 32] {
        let grade = ColourGrade::single(Arc::new(ColourCube::identity(size).unwrap()), 1.0);
        for blue in [0u8, 51, 128, 204, 255] {
            let pixels = run(&gpu, &mut pass, blue, Some(&grade));
            for y in 0..16usize {
                for x in 0..16usize {
                    let got = &pixels[(y * 16 + x) * 4..][..4];
                    assert!(got[0].abs_diff((x * 17) as u8) <= 1, "size {size} ({x},{y},{blue}): {got:?}");
                    assert!(got[1].abs_diff((y * 17) as u8) <= 1, "size {size} ({x},{y},{blue}): {got:?}");
                    assert!(got[2].abs_diff(blue) <= 1, "size {size} ({x},{y},{blue}): {got:?}");
                    assert_eq!(got[3], 255);
                }
            }
        }
    }
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn a_cube_looks_up_like_the_cpu_reference() {
    let _lock = serial();
    let gpu = Gpu::new(wgpu::Backends::VULKAN);
    let mut pass = ColourCubePass::new(&gpu.device);
    let grade = ColourGrade::single(Arc::new(bent(16)), 1.0);
    for blue in [0u8, 34, 119, 221, 255] {
        let pixels = run(&gpu, &mut pass, blue, Some(&grade));
        assert_matches_reference(&pixels, blue, &grade, 2);
    }
    // A cube of another size, and a strength between.
    let half = ColourGrade::single(Arc::new(bent(32)), 0.5);
    let pixels = run(&gpu, &mut pass, 85, Some(&half));
    assert_matches_reference(&pixels, 85, &half, 2);
    assert_ne!(&pixels[..3], &[0, 0, 85], "the grade does something");
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn two_cubes_blend_and_a_neutral_grade_draws_nothing() {
    let _lock = serial();
    let gpu = Gpu::new(wgpu::Backends::VULKAN);
    let mut pass = ColourCubePass::new(&gpu.device);
    let blend = ColourGrade {
        from: Arc::new(ColourCube::identity(16).unwrap()),
        to: Some(Arc::new(bent(16))),
        mix: 0.4,
        strength: 1.0,
    };
    let pixels = run(&gpu, &mut pass, 170, Some(&blend));
    assert_matches_reference(&pixels, 170, &blend, 2);
    assert!(pass.is_active());
    let zero = ColourGrade { strength: 0.0, ..blend };
    pass.set(&gpu.device, &gpu.queue, Some(&zero));
    assert!(!pass.is_active());
    pass.set(&gpu.device, &gpu.queue, None);
    assert!(!pass.is_active());
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn only_a_bounded_number_of_cubes_stay_uploaded() {
    let _lock = serial();
    let gpu = Gpu::new(wgpu::Backends::VULKAN);
    let mut pass = ColourCubePass::new(&gpu.device);
    for shade in 0..(RESIDENT_CUBES as u8 + 6) {
        let rgb = vec![shade; 2 * 2 * 2 * 3];
        let grade = ColourGrade::single(Arc::new(ColourCube::new(2, rgb).unwrap()), 1.0);
        pass.set(&gpu.device, &gpu.queue, Some(&grade));
    }
    assert_eq!(pass.resident(), RESIDENT_CUBES);
    // The same cube again uploads nothing new.
    let again = ColourGrade::single(Arc::new(ColourCube::new(2, vec![21; 24]).unwrap()), 1.0);
    pass.set(&gpu.device, &gpu.queue, Some(&again));
    assert_eq!(pass.resident(), RESIDENT_CUBES);
}
