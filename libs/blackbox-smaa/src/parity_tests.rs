//! GPU parity with the `smaa` crate 0.21.0 this port follows.
//!
//! Both implementations run on the same synthetic image of aliased hard edges, on the same wgpu device, into an
//! `Rgba8Unorm` output. The outputs must agree to within [`MAX_CHANNEL_DIFF`] per channel, with at least
//! [`MIN_IDENTICAL`] of the pixels equal, and SMAA must have changed some edge pixels, so a pass that does nothing
//! cannot pass.
//!
//! The GPU test is `#[ignore]`d like the other GPU tests in `blackbox-render`, so `cargo test` stays GPU-free. Run it
//! with `cargo test -p blackbox-smaa -- --ignored`. It skips with a note when no adapter is found.

use std::time::Instant;

use crate::area::{AREA_SIZE, area_texture};
use crate::{SmaaMode, SmaaTarget};

const WIDTH: u32 = 256;
const HEIGHT: u32 = 144;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// The largest per-channel difference allowed between the two outputs.
const MAX_CHANNEL_DIFF: i32 = 2;
/// The fraction of pixels that must be identical in every channel.
const MIN_IDENTICAL: f64 = 0.995;

/// Copies the source texture into the SMAA frame's colour view, texel for texel, with `textureLoad`.
const COPY_WGSL: &str = "
@group(0) @binding(0) var source: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    return textureLoad(source, vec2<i32>(position.xy), 0);
}
";

/// Times the CPU generation of the area table. Not GPU-bound, so it runs in a plain `cargo test`.
#[test]
fn area_table_generation_is_timed() {
    let start = Instant::now();
    let table = area_texture();
    let elapsed = start.elapsed();
    eprintln!("area_texture() generated {} bytes in {elapsed:?}", table.len());
    let (width, height) = AREA_SIZE;
    assert_eq!(table.len(), width as usize * height as usize * 2);
}

/// A white field with a rotated black square, a 1-px horizontal and vertical line, and a 1-px diagonal. Every edge
/// is aliased, at several orientations.
fn synthetic_scene() -> Vec<u8> {
    let mut pixels = vec![255u8; (WIDTH * HEIGHT * 4) as usize];
    let mut put = |x: u32, y: u32| {
        let index = ((y * WIDTH + x) * 4) as usize;
        pixels[index..index + 4].copy_from_slice(&[0, 0, 0, 255]);
    };

    let (cx, cy, half, angle) = (128.0_f32, 72.0_f32, 40.0_f32, 0.5_f32);
    let (sin, cos) = angle.sin_cos();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let u = cos * dx + sin * dy;
            let v = -sin * dx + cos * dy;
            if u.abs() <= half && v.abs() <= half {
                put(x, y);
            }
        }
    }
    for x in 10..240 {
        put(x, 10);
    }
    for y in 100..140 {
        put(230, y);
    }
    for i in 0..=245u32 {
        let slope = i as f32 * 55.0 / 245.0;
        put(5 + i, 5 + slope.round() as u32);
    }
    pixels
}

/// A device and queue, or `None` (with a note on stderr) when this machine has no adapter.
struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
        apply_limit_buckets: false,
    }));
    let adapter = match adapter {
        Ok(adapter) => adapter,
        Err(e) => {
            eprintln!("skipping: no GPU adapter ({e:?})");
            return None;
        }
    };
    eprintln!("SMAA parity on {}", adapter.get_info().name);
    let (device, queue) = match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("skipping: no device ({e:?})");
            return None;
        }
    };
    Some(Gpu { device, queue })
}

fn extent() -> wgpu::Extent3d {
    wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 }
}

fn upload(gpu: &Gpu, pixels: &[u8]) -> wgpu::Texture {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("smaa.parity.input"),
        size: extent(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    gpu.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(WIDTH * 4), rows_per_image: Some(HEIGHT) },
        extent(),
    );
    texture
}

fn output_texture(gpu: &Gpu, label: &str) -> wgpu::Texture {
    gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: extent(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// Draws the input texture into a colour view with a fullscreen triangle.
struct CopyPass {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

impl CopyPass {
    fn new(device: &wgpu::Device, source: &wgpu::TextureView) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("smaa.parity.copy"),
            source: wgpu::ShaderSource::Wgsl(COPY_WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("smaa.parity.copy.layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("smaa.parity.copy.bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(source) }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("smaa.parity.copy.pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("smaa.parity.copy.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline, bind_group }
    }

    fn draw_into(&self, gpu: &Gpu, target: &wgpu::TextureView) {
        let mut encoder =
            gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("smaa.parity.copy") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("smaa.parity.copy.pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        gpu.queue.submit([encoder.finish()]);
    }
}

/// Reads an `Rgba8Unorm` texture of the test size back, tightly packed.
fn read_back(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u8> {
    let row = (WIDTH * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("smaa.parity.readback"),
        size: u64::from(row * HEIGHT),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder =
        gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("smaa.parity.readback") });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(HEIGHT) },
        },
        extent(),
    );
    gpu.queue.submit([encoder.finish()]);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    gpu.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let mapped = slice.get_mapped_range().unwrap();
    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for y in 0..HEIGHT as usize {
        let start = y * row as usize;
        pixels.extend_from_slice(&mapped[start..start + (WIDTH * 4) as usize]);
    }
    pixels
}

/// The `smaa` crate 0.21.0, the reference.
fn reference_smaa(gpu: &Gpu, copy: &CopyPass) -> Vec<u8> {
    let output = output_texture(gpu, "smaa.parity.reference");
    let output_view = output.create_view(&Default::default());
    let mut target = smaa::SmaaTarget::new(&gpu.device, &gpu.queue, WIDTH, HEIGHT, FORMAT, smaa::SmaaMode::Smaa1X);
    let frame = target.start_frame(&gpu.device, &gpu.queue, &output_view);
    copy.draw_into(gpu, &frame);
    frame.resolve();
    read_back(gpu, &output)
}

/// This crate's SMAA 1x, built the same way the renderer builds it.
fn ported_smaa(gpu: &Gpu, copy: &CopyPass) -> Vec<u8> {
    let output = output_texture(gpu, "smaa.parity.ported");
    let output_view = output.create_view(&Default::default());
    let mut target = SmaaTarget::new(&gpu.device, &gpu.queue, WIDTH, HEIGHT, FORMAT, SmaaMode::Smaa1X);
    let frame = target.start_frame(&gpu.device, &gpu.queue, &output_view);
    copy.draw_into(gpu, &frame);
    frame.resolve();
    read_back(gpu, &output)
}

#[test]
#[ignore = "needs a GPU"]
fn smaa_matches_the_smaa_crate_on_hard_edges() {
    let Some(gpu) = gpu() else { return };
    let scene = synthetic_scene();
    let input = upload(&gpu, &scene);
    let input_view = input.create_view(&Default::default());
    let copy = CopyPass::new(&gpu.device, &input_view);

    let reference = reference_smaa(&gpu, &copy);
    let ported = ported_smaa(&gpu, &copy);
    assert_eq!(reference.len(), scene.len());
    assert_eq!(ported.len(), scene.len());

    let max_diff = ported.iter().zip(&reference).map(|(a, b)| (i32::from(*a) - i32::from(*b)).abs()).max().unwrap_or(0);
    let total = (WIDTH * HEIGHT) as usize;
    let identical = ported.as_chunks::<4>().0.iter().zip(reference.as_chunks::<4>().0).filter(|(a, b)| a == b).count();
    let changed = reference.as_chunks::<4>().0.iter().zip(scene.as_chunks::<4>().0).filter(|(a, b)| a != b).count();
    eprintln!(
        "SMAA parity: max channel diff {max_diff}, {identical}/{total} pixels identical, \
         {changed} pixels changed from the input by the smaa crate"
    );
    assert!(max_diff <= MAX_CHANNEL_DIFF, "max channel difference {max_diff} exceeds {MAX_CHANNEL_DIFF}");
    assert!(
        identical as f64 >= total as f64 * MIN_IDENTICAL,
        "only {identical} of {total} pixels are identical (need {MIN_IDENTICAL})"
    );
    assert!(changed > 0, "the smaa crate changed no pixel of the input: the comparison would be vacuous");
}

/// The same passes, with the output view given only once the scene is drawn (a swapchain image acquired late).
fn late_output_smaa(gpu: &Gpu, copy: &CopyPass) -> Vec<u8> {
    let output = output_texture(gpu, "smaa.parity.late");
    let output_view = output.create_view(&Default::default());
    let target = SmaaTarget::new(&gpu.device, &gpu.queue, WIDTH, HEIGHT, FORMAT, SmaaMode::Smaa1X);
    copy.draw_into(gpu, target.input_view().expect("an enabled target has an input view"));
    let mut encoder =
        gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("smaa.parity.late") });
    target.encode_to(&mut encoder, &output_view);
    gpu.queue.submit([encoder.finish()]);
    read_back(gpu, &output)
}

#[test]
#[ignore = "needs a GPU"]
fn an_output_view_given_late_gets_the_same_image() {
    let Some(gpu) = gpu() else { return };
    let input = upload(&gpu, &synthetic_scene());
    let input_view = input.create_view(&Default::default());
    let copy = CopyPass::new(&gpu.device, &input_view);
    assert_eq!(late_output_smaa(&gpu, &copy), ported_smaa(&gpu, &copy));
}
