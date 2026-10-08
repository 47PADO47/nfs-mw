//! GPU checks: opaque occlusion, world-unit fading and replacement depth targets.

use glam::{Mat4, Vec3};

use super::{
    resources::{self, Globals, Shared},
    soft_particles::SoftParticles,
};
use crate::{DEFAULT_SOFT_DISTANCE, EffectLayer};

#[test]
#[ignore = "needs a Vulkan GPU"]
fn soft_particles_depth_vulkan() {
    check(wgpu::Backends::VULKAN);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "needs a Direct3D 12 GPU"]
fn soft_particles_depth_dx12() {
    check(wgpu::Backends::DX12);
}

fn check(backend: wgpu::Backends) {
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
    let shared = Shared::new(&device);
    let projection = glam::camera::rh::proj::directx::perspective_infinite_reverse(1.0, 1.0, 0.1);
    let globals = Globals {
        view_proj: projection.to_cols_array_2d(),
        camera_pos: [0.0, 0.0, 0.0, 1.0],
        light_dir: [0.0; 4],
        fog_color: [0.0; 4],
        fog_range: [f32::MAX, f32::MAX, 0.0, 0.0],
    };
    queue.write_buffer(&shared.globals, 0, bytemuck::bytes_of(&globals));
    let mut soft = SoftParticles::new(&device, wgpu::TextureFormat::Rgba8Unorm, &shared);
    let mut vertices = Vec::new();
    EffectLayer::particle_quad(
        &mut vertices,
        [
            Vec3::new(-1.0, -1.0, -5.0),
            Vec3::new(1.0, -1.0, -5.0),
            Vec3::new(1.0, 1.0, -5.0),
            Vec3::new(-1.0, 1.0, -5.0),
        ],
        [255; 4],
        [0.5, 1.5],
    );
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("test particle"),
        size: std::mem::size_of_val(vertices.as_slice()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&vertices));
    let mut draw =
        |distance, width| sample(&device, &queue, &shared, &mut soft, &buffer, projection, (distance, width));
    let sky = draw(0.0, 64);
    let far = draw(6.0, 64);
    let near = draw(5.05, 64);
    let equal = draw(5.0, 64);
    let occluded = draw(4.0, 64);
    let resized = draw(6.0, 128);
    eprintln!("{backend:?}: sky={sky}, far={far}, near={near}, equal={equal}, occluded={occluded}, resized={resized}");
    assert!(far > 20 && sky.abs_diff(far) <= 1);
    assert!(near > 0 && near < far / 2, "nearby geometry must soften alpha");
    assert_eq!(equal, 0, "intersection vanishes at the opaque depth");
    assert_eq!(occluded, 0, "opaque geometry must hide particles behind it");
    assert!(far.abs_diff(resized) <= 4, "depth binding must follow the replacement target dimensions");
}

fn sample(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    shared: &Shared,
    soft: &mut SoftParticles,
    buffer: &wgpu::Buffer,
    projection: Mat4,
    case: (f32, u32),
) -> u8 {
    let (distance, width) = case;
    let size = wgpu::Extent3d { width, height: width, depth_or_array_layers: 1 };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("test target"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = resources::create_depth(device, width, width);
    let clear_depth = if distance == 0.0 { 0.0 } else { projection.project_point3(Vec3::new(0.0, 0.0, -distance)).z };
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("test opaque plane"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_depth),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    soft.prepare(device, queue, &depth, projection.inverse(), DEFAULT_SOFT_DISTANCE);
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("test soft particle"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        soft.bind(&mut pass, shared);
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..6, 0..1);
    }
    let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("test readback"),
        size: u64::from(row * width),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(width) },
        },
        size,
    );
    queue.submit([encoder.finish()]);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let pixels = slice.get_mapped_range().unwrap();
    pixels[(width / 2 * row + width / 2 * 4) as usize]
}
