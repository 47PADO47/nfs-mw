//! Headless GPU checks of single passes against a CPU model, on inputs made of numbers rather than a
//! scene: the dilated depth and vectors, the luma, the exposure, the shading change and the accumulation.

mod common;

use common::Gpu;
use fsr3_wgpu::{
    DebugTexture, Fsr3Config, Fsr3Context, Fsr3Inputs, Fsr3Outputs, device_to_view_depth, jitter::offset, view_depth,
};
use glam::{UVec2, Vec2};
use wgpu::util::DeviceExt;

const RENDER: UVec2 = UVec2::new(16, 12);
const OUTPUT: UVec2 = UVec2::new(32, 24);
const NEAR: f32 = 0.1;
const FOV: f32 = 1.0;

/// A context fed with images made on the CPU.
struct Raw {
    gpu: Gpu,
    ctx: Fsr3Context,
    output: wgpu::Texture,
    exposure: Option<f32>,
}

fn texture(raw: &Raw, size: UVec2, format: wgpu::TextureFormat, data: &[u8]) -> wgpu::TextureView {
    raw.gpu
        .device
        .create_texture_with_data(
            &raw.gpu.queue,
            &wgpu::TextureDescriptor {
                label: Some("input"),
                size: wgpu::Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data,
        )
        .create_view(&wgpu::TextureViewDescriptor::default())
}

impl Raw {
    fn new(config: Fsr3Config) -> Self {
        let gpu = Gpu::new();
        let ctx =
            Fsr3Context::new(&gpu.device, Fsr3Config { output_format: wgpu::TextureFormat::Rgba32Float, ..config })
                .unwrap();
        let output = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("output"),
            size: wgpu::Extent3d { width: OUTPUT.x, height: OUTPUT.y, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Self { gpu, ctx, output, exposure: None }
    }

    /// One dispatch with the given images (row-major, `RENDER` sized).
    fn dispatch(&mut self, color: &[[f32; 4]], depth: &[f32], motion: &[[f32; 2]], jitter: Vec2, dt: f32, reset: bool) {
        let color = texture(self, RENDER, wgpu::TextureFormat::Rgba32Float, bytemuck::cast_slice(color));
        let depth = texture(self, RENDER, wgpu::TextureFormat::R32Float, bytemuck::cast_slice(depth));
        let motion = texture(self, RENDER, wgpu::TextureFormat::Rg32Float, bytemuck::cast_slice(motion));
        let exposure =
            self.exposure.map(|e| texture(self, UVec2::ONE, wgpu::TextureFormat::R32Float, bytemuck::bytes_of(&e)));
        let output = self.output.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.ctx
            .dispatch(
                &self.gpu.device,
                &self.gpu.queue,
                &mut encoder,
                &Fsr3Inputs {
                    color: &color,
                    depth: &depth,
                    motion_vectors: &motion,
                    exposure: exposure.as_ref(),
                    reactive: None,
                    transparency_and_composition: None,
                    render_size: RENDER,
                    jitter,
                    motion_vector_scale: Vec2::ONE,
                    delta_time: dt,
                    pre_exposure: 0.0,
                    sharpness: None,
                    camera_near: NEAR,
                    camera_far: f32::INFINITY,
                    camera_fov_y: FOV,
                    view_space_to_meters: 1.0,
                    reset,
                },
                &Fsr3Outputs { output: &output, size: OUTPUT },
            )
            .unwrap();
        self.gpu.queue.submit([encoder.finish()]);
    }

    fn read(&self, which: DebugTexture) -> (UVec2, usize, Vec<f32>) {
        let texture = self.ctx.debug_texture(which).unwrap();
        let size = UVec2::new(texture.width(), texture.height());
        let (texel, channels) = match texture.format() {
            wgpu::TextureFormat::R32Float => (4, 1),
            wgpu::TextureFormat::Rg32Float => (8, 2),
            wgpu::TextureFormat::Rgba16Float => (8, 4),
            other => panic!("{other:?}"),
        };
        let bytes = self.gpu.read_bytes(texture, size, texel);
        let values = if channels == 4 {
            bytemuck::cast_slice::<u8, u16>(&bytes).iter().map(|h| common::decode_f16(*h)).collect()
        } else {
            bytemuck::cast_slice::<u8, f32>(&bytes).to_vec()
        };
        (size, channels, values)
    }

    fn frame_info(&self) -> [f32; 4] {
        let bytes = self.gpu.read_buffer(self.ctx.debug_frame_info().unwrap(), 16);
        bytemuck::cast_slice::<u8, f32>(&bytes).try_into().unwrap()
    }
}

/// A repeatable stream of numbers in `[0, 1)`.
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f32) / 16_777_216.0
    }
}

fn index(x: i32, y: i32) -> usize {
    (y as u32 * RENDER.x + x as u32) as usize
}

fn assert_close(what: &str, got: f32, want: f32, tolerance: f32) {
    assert!((got - want).abs() <= tolerance * want.abs().max(1.0), "{what}: {got} vs {want}");
}

#[test]
#[ignore = "needs a GPU"]
fn the_prepared_inputs_match_a_cpu_model() {
    let mut raw = Raw::new(Fsr3Config::new());
    let mut noise = Noise(7);
    let n = (RENDER.x * RENDER.y) as usize;
    let color: Vec<[f32; 4]> = (0..n).map(|_| [noise.next() * 4.0, noise.next() * 2.0, noise.next(), 1.0]).collect();
    let depth: Vec<f32> = (0..n).map(|_| 0.001 + noise.next() * 0.9).collect();
    let motion: Vec<[f32; 2]> = (0..n).map(|_| [noise.next() * 6.0 - 3.0, noise.next() * 6.0 - 3.0]).collect();
    raw.dispatch(&color, &depth, &motion, Vec2::ZERO, 1.0 / 60.0, true);

    let factors =
        device_to_view_depth(fsr3_wgpu::DepthConvention::REVERSE_INFINITE, NEAR, f32::INFINITY, FOV, RENDER).unwrap();
    let (_, _, luma) = raw.read(DebugTexture::Luma);
    let (_, _, dilated_depth) = raw.read(DebugTexture::DilatedDepth);
    let (_, _, dilated_motion) = raw.read(DebugTexture::DilatedMotionVectors);
    let (_, _, farthest) = raw.read(DebugTexture::FarthestDepth);
    let (half, _, farthest_mip1) = raw.read(DebugTexture::FarthestDepthMip1);

    // The neighbours in the order of AMD's loop; with reverse depth the nearest has the largest value.
    let offsets = [(1, 0), (0, 1), (0, -1), (-1, 0), (-1, 1), (1, 1), (-1, -1), (1, -1)];
    let mut expected_farthest = vec![0.0_f32; n];
    for y in 0..RENDER.y as i32 {
        for x in 0..RENDER.x as i32 {
            let i = index(x, y);
            let [r, g, b, _] = color[i];
            assert_close("luma", luma[i], 0.2126 * r + 0.7152 * g + 0.0722 * b, 1e-5);

            let (mut nearest, mut coord, mut far) = (depth[i], (x, y), depth[i]);
            for (dx, dy) in offsets {
                let (px, py) = (x + dx, y + dy);
                if px < 0 || py < 0 || px >= RENDER.x as i32 || py >= RENDER.y as i32 {
                    continue;
                }
                let d = depth[index(px, py)];
                if d > nearest {
                    far = far.min(d);
                    coord = (px, py);
                    nearest = d;
                }
            }
            assert_eq!(dilated_depth[i], nearest, "dilated depth at {x},{y}");
            let m = motion[index(coord.0, coord.1)];
            assert_close("dilated motion x", dilated_motion[i * 2], m[0] / RENDER.x as f32, 1e-5);
            assert_close("dilated motion y", dilated_motion[i * 2 + 1], m[1] / RENDER.y as f32, 1e-5);
            expected_farthest[i] = view_depth(factors, far).min(65504.0);
            assert_close("farthest depth in metres", farthest[i], expected_farthest[i], 1e-4);
        }
    }
    // The half resolution image holds the mean of 2x2 blocks.
    for y in 0..half.y as i32 {
        for x in 0..half.x as i32 {
            let mean = [(0, 0), (1, 0), (0, 1), (1, 1)]
                .iter()
                .map(|(dx, dy)| expected_farthest[index(2 * x + dx, 2 * y + dy)])
                .sum::<f32>()
                / 4.0;
            assert_close("farthest depth mip 1", farthest_mip1[(y as u32 * half.x + x as u32) as usize], mean, 1e-4);
        }
    }
}

/// The exposure of AMD's auto exposure for a smoothed average log luminance.
fn auto_exposure(log_average: f32) -> f32 {
    let average = log_average.exp();
    let ev = (average * 100.0 / 12.5).log2();
    1.0 / (78.0 / (0.65 * 100.0) * 2f32.powf(ev))
}

fn flat(value: f32) -> (Vec<[f32; 4]>, Vec<f32>, Vec<[f32; 2]>) {
    let n = (RENDER.x * RENDER.y) as usize;
    (vec![[value; 4]; n], vec![0.05; n], vec![[0.0; 2]; n])
}

#[test]
#[ignore = "needs a GPU"]
fn auto_exposure_follows_the_average_luminance_and_eases_in_time() {
    let mut raw = Raw::new(Fsr3Config { auto_exposure: true, ..Fsr3Config::new() });
    let (color, depth, motion) = flat(4.0);
    raw.dispatch(&color, &depth, &motion, Vec2::ZERO, 1.0 / 60.0, true);
    let info = raw.frame_info();
    // The first frame (a reset) takes the frame's own average.
    assert_close("log luma", info[1], 4.0_f32.ln(), 1e-4);
    assert_close("average luma", info[2], 4.0, 1e-4);
    assert_close("exposure", info[0], auto_exposure(4.0_f32.ln()), 1e-3);

    // A brighter frame moves the average a fraction of the way, by 1 - exp(-dt).
    let (color, depth, motion) = flat(16.0);
    raw.dispatch(&color, &depth, &motion, Vec2::ZERO, 0.1, false);
    let info = raw.frame_info();
    let smoothed = 4.0_f32.ln() + (16.0_f32.ln() - 4.0_f32.ln()) * (1.0 - (-0.1_f32).exp());
    assert_close("smoothed log luma", info[1], smoothed, 1e-4);
    assert_close("exposure", info[0], auto_exposure(smoothed), 1e-3);

    // A dark frame cannot take the average below zero (the log luma is floored at 0 after smoothing).
    let (color, depth, motion) = flat(0.01);
    raw.dispatch(&color, &depth, &motion, Vec2::ZERO, 1.0, true);
    let info = raw.frame_info();
    assert!(info[1] >= 0.0 && info[1] < 1e-3, "{}", info[1]);
    assert_close("exposure of a dark frame", info[0], auto_exposure(0.0), 2e-2);
}

#[test]
#[ignore = "needs a GPU"]
fn the_exposure_input_is_used_when_auto_exposure_is_off() {
    let mut raw = Raw::new(Fsr3Config::new());
    let (color, depth, motion) = flat(4.0);
    for (input, expected) in [(Some(0.5), 0.5), (Some(0.0), 1.0), (None, 1.0), (Some(7.25), 7.25)] {
        raw.exposure = input;
        raw.dispatch(&color, &depth, &motion, Vec2::ZERO, 1.0 / 60.0, false);
        assert_close(&format!("exposure for {input:?}"), raw.frame_info()[0], expected, 1e-6);
    }
}

fn gradient(scale: f32) -> Vec<[f32; 4]> {
    (0..RENDER.y)
        .flat_map(|y| {
            (0..RENDER.x).map(move |x| {
                let v = (0.3 + 0.01 * x as f32 + 0.005 * y as f32) * scale;
                [v, v, v, 1.0]
            })
        })
        .collect()
}

fn interior_mean(size: UVec2, channels: usize, channel: usize, values: &[f32], margin: u32) -> f32 {
    let mut sum = 0.0;
    let mut count = 0.0;
    for y in margin..size.y - margin {
        for x in margin..size.x - margin {
            sum += values[((y * size.x + x) as usize) * channels + channel];
            count += 1.0;
        }
    }
    sum / count
}

#[test]
#[ignore = "needs a GPU"]
fn shading_change_sees_a_brightness_change_and_not_a_static_scene() {
    let (_, depth, motion) = flat(0.0);
    let mut raw = Raw::new(Fsr3Config::new());
    raw.dispatch(&gradient(1.0), &depth, &motion, Vec2::ZERO, 1.0 / 60.0, true);
    raw.dispatch(&gradient(1.0), &depth, &motion, Vec2::ZERO, 1.0 / 60.0, false);
    let (size, channels, values) = raw.read(DebugTexture::ShadingChange);
    let still = interior_mean(size, channels, 0, &values, 1);
    assert!(still < 0.01, "a static scene has no shading change: {still}");

    // The scene gets 1.5 times brighter: the relative difference 1 - 1 / 1.5.
    raw.dispatch(&gradient(1.5), &depth, &motion, Vec2::ZERO, 1.0 / 60.0, false);
    let (size, channels, values) = raw.read(DebugTexture::ShadingChange);
    let brighter = interior_mean(size, channels, 0, &values, 1);
    assert!((brighter - 1.0 / 3.0).abs() < 0.04, "a 1.5x brightness change reads as about a third: {brighter}");

    // A halving of the brightness gives 0.5, and the shading change takes that share of the
    // accumulation the pixels had (the stored value, one third more than the frame's own).
    let (size, channels, stored) = raw.read(DebugTexture::Accumulation);
    let before = interior_mean(size, channels, 0, &stored, 2);
    raw.dispatch(&gradient(0.75), &depth, &motion, Vec2::ZERO, 1.0 / 60.0, false);
    let (size, channels, values) = raw.read(DebugTexture::ShadingChange);
    let darker = interior_mean(size, channels, 0, &values, 1);
    assert!((darker - 0.5).abs() < 0.05, "halving reads as half: {darker}");
    let (size, channels, masks) = raw.read(DebugTexture::ReactiveMasks);
    let after = interior_mean(size, channels, 3, &masks, 2);
    assert!(
        (after - before * (1.0 - darker)).abs() < 0.05,
        "accumulation {before} -> {after} by a shading change of {darker}"
    );
}

#[test]
#[ignore = "needs a GPU"]
fn accumulation_grows_by_a_third_per_frame_on_a_static_scene() {
    let (_, depth, motion) = flat(0.0);
    let mut raw = Raw::new(Fsr3Config::new());
    let mut seen = Vec::new();
    for frame in 0..6 {
        raw.dispatch(&gradient(1.0), &depth, &motion, offset(frame, 8), 1.0 / 60.0, frame == 0);
        let (size, channels, masks) = raw.read(DebugTexture::ReactiveMasks);
        seen.push(interior_mean(size, channels, 3, &masks, 2));
    }
    let want = [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0, 1.0, 1.0];
    for (got, want) in seen.iter().zip(want) {
        assert!((got - want).abs() < 0.02, "{seen:?}");
    }
}
