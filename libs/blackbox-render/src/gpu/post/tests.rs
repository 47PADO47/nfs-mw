//! GPU checks of the post-process chain on a real backend (no surface or assets).

use std::{cell::RefCell, rc::Rc};

use super::{PassContext, PassIo, PostChain, PostPass};
use crate::gpu::targets::{FrameTargets, HDR_FORMAT};

const OUTPUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[test]
#[ignore = "needs a Vulkan GPU"]
fn post_chain_vulkan() {
    check(wgpu::Backends::VULKAN);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "needs a Direct3D 12 GPU"]
fn post_chain_dx12() {
    check(wgpu::Backends::DX12);
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Gpu {
    fn new(backend: wgpu::Backends) -> Self {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = backend;
        let instance = wgpu::Instance::new(desc);
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .expect("test GPU adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        Self { device, queue }
    }

    /// Clear a fresh scene image of `size` to `color` and run `chain` into an output of `out` size.
    fn run(&self, chain: &mut PostChain, size: (u32, u32), color: wgpu::Color, out: (u32, u32)) -> Vec<[u8; 4]> {
        let device = &self.device;
        let scene = FrameTargets::new(device, HDR_FORMAT, size);
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("post test output"),
            size: wgpu::Extent3d { width: out.0, height: out.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OUTPUT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("post test scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &scene.color_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(color), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        }));
        let ctx = PassContext { device, queue: &self.queue, scene: &scene, output_size: out };
        chain.encode(&ctx, &mut encoder, (&view, OUTPUT_FORMAT));
        let row = (out.0 * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post test readback"),
            size: u64::from(row * out.1),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(out.1),
                },
            },
            target.size(),
        );
        self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let mapped = slice.get_mapped_range().unwrap();
        let mut pixels = Vec::new();
        for y in 0..out.1 as usize {
            let line = &mapped[y * row as usize..][..out.0 as usize * 4];
            pixels.extend(line.as_chunks::<4>().0.iter().copied());
        }
        pixels
    }
}

/// Writes a constant colour and records that it ran, and that the depth view can be sampled.
struct Constant {
    colour: wgpu::Color,
    log: Rc<RefCell<Vec<&'static str>>>,
    name: &'static str,
}

impl PostPass for Constant {
    fn name(&self) -> &'static str {
        self.name
    }

    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        self.log.borrow_mut().push(self.name);
        let layout = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("depth sampling"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let _depth = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("depth sampling"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&ctx.scene.depth_view),
            }],
        });
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("constant"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: io.output,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(self.colour), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        }));
    }
}

fn near(pixel: [u8; 4], expected: [u8; 4]) -> bool {
    pixel.iter().zip(expected).all(|(&a, b)| a.abs_diff(b) <= 1)
}

fn check(backend: wgpu::Backends) {
    let gpu = Gpu::new(backend);
    let hdr = wgpu::Color { r: 1.5, g: 0.5, b: 0.25, a: 0.3 };

    // The resolve pass alone clamps HDR to the output range and forces opaque alpha.
    let mut chain = PostChain::new(&gpu.device);
    assert_eq!(chain.names(), ["resolve"]);
    let same = gpu.run(&mut chain, (4, 4), hdr, (4, 4));
    assert!(same.iter().all(|&p| near(p, [255, 128, 64, 255])), "{same:?}");

    // A different output size filters (a flat image stays flat).
    let scaled = gpu.run(&mut chain, (2, 2), hdr, (8, 8));
    assert_eq!(scaled.len(), 64);
    assert!(scaled.iter().all(|&p| near(p, [255, 128, 64, 255])), "{scaled:?}");

    // Inserted passes run in order before the resolve pass, through the scratch images.
    let log = Rc::new(RefCell::new(Vec::new()));
    let pass = |name, g| {
        Box::new(Constant { colour: wgpu::Color { r: 0.0, g, b: 0.0, a: 1.0 }, log: log.clone(), name })
            as Box<dyn PostPass>
    };
    chain.insert(0, pass("first", 0.25));
    chain.insert(99, pass("second", 0.5));
    assert_eq!(chain.names(), ["first", "second", "resolve"]);
    let chained = gpu.run(&mut chain, (4, 4), hdr, (4, 4));
    assert_eq!(*log.borrow(), ["first", "second"]);
    assert!(
        chained.iter().all(|&p| near(p, [0, 128, 0, 255])),
        "the last inserted pass feeds the resolve: {chained:?}"
    );
}
