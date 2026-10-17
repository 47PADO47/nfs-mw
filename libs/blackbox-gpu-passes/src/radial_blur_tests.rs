//! Checks of the radial blur: the tap math on the CPU, the shader through naga, and the pass on a real backend
//! (ignored without a GPU).

use crate::filter_source;
use crate::radial_blur::{
    RADIAL_MAX_TAPS, RadialBlur, RadialBlurIo, RadialBlurSettings, params, reach, tap_position, tap_weight,
};
use crate::test_support::{Gpu, serial};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[test]
fn the_first_sample_is_the_pixel_and_the_last_reaches_the_centre() {
    for taps in 2..=RADIAL_MAX_TAPS {
        assert_eq!(tap_position(0, taps), 0.0);
        assert!((tap_position(taps - 1, taps) - 1.0).abs() < 1e-6, "{taps} taps");
    }
    assert_eq!(tap_position(0, 1), 0.0, "one sample is the pixel");
}

#[test]
fn normalised_tap_weights_sum_to_one() {
    for taps in 1..=RADIAL_MAX_TAPS {
        let p = params(RadialBlurSettings { strength: 1.0, taps, ..Default::default() });
        let sum: f32 = (0..taps).map(|i| tap_weight(i, taps) * p.a[3]).sum();
        assert!((sum - 1.0).abs() < 1e-5, "{taps} taps: {sum}");
    }
}

#[test]
fn the_pixel_counts_for_more_than_the_far_end() {
    assert!(tap_weight(0, 8) > tap_weight(7, 8));
    assert_eq!(tap_weight(0, 1), 1.0);
}

#[test]
fn zero_strength_is_the_identity() {
    assert!(RadialBlurSettings::default().is_identity());
    assert!(RadialBlurSettings { strength: 0.0, ..Default::default() }.is_identity());
    assert!(RadialBlurSettings { strength: -3.0, ..Default::default() }.is_identity());
    assert!(RadialBlurSettings { strength: f32::NAN, ..Default::default() }.is_identity());
    assert!(!RadialBlurSettings { strength: 0.1, ..Default::default() }.is_identity());
}

#[test]
fn settings_are_clamped_and_non_finite_values_take_the_defaults() {
    let wild = RadialBlurSettings { strength: 7.0, centre: [f32::NAN, 2.0], inner: 5.0, taps: 1000 };
    let clean = wild.sanitized();
    assert_eq!(clean.strength, 1.0);
    assert_eq!(clean.centre, [0.5, 1.0]);
    assert_eq!(clean.inner, 0.99);
    assert_eq!(clean.taps, RADIAL_MAX_TAPS);
    assert_eq!(RadialBlurSettings { taps: 0, ..Default::default() }.sanitized().taps, 1);
}

#[test]
fn the_reach_starts_at_the_inner_radius_and_is_at_most_the_strength() {
    let s = RadialBlurSettings { strength: 0.8, inner: 0.25, ..Default::default() };
    assert_eq!(reach(s, [0.5, 0.5]), 0.0, "the centre");
    assert_eq!(reach(s, [0.5, 0.25]), 0.0, "on the inner radius");
    let mid = reach(s, [0.5, 0.1]);
    assert!(mid > 0.0 && mid < 0.8, "{mid}");
    let corner = reach(s, [0.0, 0.0]);
    assert!(corner <= 0.8 + 1e-6, "never above the strength: {corner}");
    assert!(corner > reach(s, [0.5, 0.0]), "grows outwards");
}

#[test]
fn the_parameter_block_carries_the_settings_in_the_documented_slots() {
    let p = params(RadialBlurSettings { strength: 0.5, centre: [0.3, 0.7], inner: 0.2, taps: 4 });
    assert_eq!([p.a[0], p.a[1], p.a[2]], [0.5, 0.2, 4.0]);
    assert_eq!([p.b[0], p.b[1]], [0.3, 0.7]);
}

#[test]
fn the_blur_shader_validates_and_has_its_entry_point() {
    let source = filter_source(include_str!("shaders/radial_blur.wgsl"));
    let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::default())
        .validate(&module)
        .expect("valid");
    let entries: Vec<_> = module.entry_points.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(entries, ["vs_main", "fs_radial"]);
}

/// A `size` x `size` image of vertical stripes: even columns white, odd columns black.
fn stripes(gpu: &Gpu, size: u32) -> wgpu::Texture {
    let texture = gpu.target(size);
    let bytes: Vec<u8> = (0..size * size)
        .flat_map(|i| if (i % size).is_multiple_of(2) { [255, 255, 255, 255] } else { [0, 0, 0, 255] })
        .collect();
    gpu.queue.write_texture(
        texture.as_image_copy(),
        &bytes,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(size * 4), rows_per_image: Some(size) },
        texture.size(),
    );
    texture
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn the_blur_keeps_the_centre_sharp_and_mixes_the_corners() {
    let _lock = serial();
    let gpu = Gpu::new(wgpu::Backends::VULKAN);
    let size = 64;
    let (src, out) = (stripes(&gpu, size), gpu.target(size));
    let (src_view, out_view) = (src.create_view(&Default::default()), out.create_view(&Default::default()));
    let mut blur = RadialBlur::new(&gpu.device);
    blur.set(RadialBlurSettings { strength: 1.0, ..Default::default() });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    blur.encode(
        &gpu.device,
        &gpu.queue,
        &mut encoder,
        &RadialBlurIo { input: &src_view, output: &out_view, output_format: FORMAT },
    );
    let pixels = gpu.finish(encoder, &out);
    let pixel = |x: usize, y: usize| pixels[(y * size as usize + x) * 4];
    // Well inside the inner radius (the centre), the blur does nothing: the stripes stay exact.
    for (x, y) in [(31, 31), (32, 31), (30, 32)] {
        let source = if x % 2 == 0 { 255 } else { 0 };
        assert_eq!(pixel(x, y), source, "centre texel ({x}, {y})");
    }
    // A corner texel, white in the source, is pulled towards the grey of the mix along its ray.
    let corner = pixel(0, 0);
    assert!(corner > 0 && corner < 255, "corner mixes the stripes: {corner}");
}
