//! GPU timestamps for direct and depth-prefiltered AO, including hierarchy construction and excluding readback.
//! Run with `cargo test -p blackbox-render ao_gpu_timings -- --ignored --nocapture`.

use super::{AmbientOcclusion, Draw};
use blackbox_gfx::{AoMethod, AoQuality, AoSettings, FrameParams, Projection};
use glam::{Mat4, Vec3};

#[test]
#[ignore = "needs a Vulkan GPU with timestamp queries; diagnostic benchmark"]
fn ao_gpu_timings() {
    let _serial = crate::gpu::test_support::serial();
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = wgpu::Backends::VULKAN;
    let instance = wgpu::Instance::new(desc);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .expect("benchmark adapter");
    let features = wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
    if !adapter.features().contains(features) {
        eprintln!("skipping AO benchmark: timestamp queries unavailable");
        return;
    }
    let (device, queue) = pollster::block_on(
        adapter.request_device(&wgpu::DeviceDescriptor { required_features: features, ..Default::default() }),
    )
    .unwrap();
    eprintln!(
        "AO benchmark: {} ({:?}), synthetic depth, AO passes only",
        adapter.get_info().name,
        adapter.get_info().backend
    );
    let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
        label: Some("AO timing"),
        ty: wgpu::QueryType::Timestamp,
        count: 2,
    });
    let resolved = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("AO timestamps"),
        size: 16,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("AO timestamp readback"),
        size: 16,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut ao = AmbientOcclusion::new(&device, (1, 1), None);
    let method = std::env::var("BLACKBOX_AO_BENCH_METHOD")
        .unwrap_or_else(|_| "gtao".into())
        .parse::<AoMethod>()
        .expect("gtao, ssao or xegtao benchmark method");
    for size in [(1920, 1080), (3840, 2160)] {
        let normals = crate::gpu::surface::targets::Targets::new(&device, size).normals;
        let (color, depth) = fixture(&device, &queue, size);
        let frame = FrameParams {
            view: Mat4::IDENTITY,
            projection: Projection::PerspectiveInfiniteReverse {
                fov_y: 1.0,
                aspect: size.0 as f32 / size.1 as f32,
                near: 0.1,
            },
            camera_position: Vec3::ZERO,
            light_dir: Vec3::NEG_Z,
            clear_color: [0.0; 3],
            fog: None,
            scattering: None,
            surface_light: None,
            camera_cut: false,
            shadow_tuning: None,
        };
        for quality in [AoQuality::Low, AoQuality::Medium, AoQuality::High] {
            for depth_mips in [false, true] {
                let mut times = Vec::new();
                for index in 0..32 {
                    let mut encoder = device.create_command_encoder(&Default::default());
                    encoder.write_timestamp(&queries, 0);
                    ao.encode(
                        &device,
                        &queue,
                        &mut encoder,
                        Draw {
                            motion: None,
                            target: &color,
                            depth: &depth,
                            size,
                            normals: &normals,
                            frame: &frame,
                            settings: AoSettings { quality, method, depth_mips, ..Default::default() },
                        },
                    );
                    encoder.write_timestamp(&queries, 1);
                    encoder.resolve_query_set(&queries, 0..2, &resolved, 0);
                    encoder.copy_buffer_to_buffer(&resolved, 0, &readback, 0, 16);
                    queue.submit([encoder.finish()]);
                    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
                    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                    let mapped = readback.slice(..).get_mapped_range().unwrap();
                    let ticks: &[u64] = bytemuck::cast_slice(&mapped);
                    if index >= 8 {
                        times.push((ticks[1] - ticks[0]) as f64 * f64::from(queue.get_timestamp_period()) / 1e6);
                    }
                    drop(mapped);
                    readback.unmap();
                }
                times.sort_by(f64::total_cmp);
                eprintln!(
                    "{}x{} {method} {quality} depth_mips={depth_mips}: median {:.3} ms, p90 {:.3} ms (24 samples)",
                    size.0, size.1, times[12], times[21]
                );
            }
        }
    }
}

fn fixture(device: &wgpu::Device, queue: &wgpu::Queue, size: (u32, u32)) -> (wgpu::TextureView, wgpu::TextureView) {
    let texture = |format, usage| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("AO benchmark fixture"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
            .create_view(&Default::default())
    };
    let color = texture(wgpu::TextureFormat::R8Unorm, wgpu::TextureUsages::RENDER_ATTACHMENT);
    let depth = texture(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("AO benchmark depth"),
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {{
                let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
                return vec4<f32>(p * 2.0 - 1.0, 0.5, 1.0);
            }}
            @fragment fn fs(@builtin(position) p: vec4<f32>) -> @builtin(frag_depth) f32 {{
                let uv = p.xy / vec2<f32>({}.0, {}.0);
                if uv.y < 0.25 {{ return 0.0; }}
                if uv.x > 0.35 && uv.x < 0.65 && uv.y < 0.6 {{ return 0.1 / 6.0; }}
                return 0.1 / (3.0 + (1.0 - uv.y) * 20.0);
            }}",
                size.0, size.1
            )
            .into(),
        ),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("AO fixture"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[],
        }),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        primitive: Default::default(),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("AO fixture"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.draw(0..3, 0..1);
    }
    // Initialise the multiply target, outside the measured interval.
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store },
            })],
            ..Default::default()
        });
    }
    queue.submit([encoder.finish()]);
    (color, depth)
}
