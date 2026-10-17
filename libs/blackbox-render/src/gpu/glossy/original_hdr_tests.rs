//! Read the actual HDR scene alpha before the intentionally opaque presentation/capture boundary.
use blackbox_gfx::{BlendMode, GlossyModel, GraphicsSettings, PostSettings, RenderBackend, Tonemap};

use super::{Fixture, Scene, data, renderer};
use crate::{Renderer, gpu::test_support};

fn half(bits: u16) -> f32 {
    let sign = match bits & 0x8000 {
        0 => 1.0,
        _ => -1.0,
    };
    let mantissa = f32::from(bits & 0x03ff);
    let exponent = i32::from((bits >> 10) & 31);
    match exponent {
        0 => sign * mantissa * 2.0_f32.powi(-24),
        31 => match bits & 0x03ff {
            0 => sign * f32::INFINITY,
            _ => f32::NAN,
        },
        _ => sign * (1.0 + mantissa / 1024.0) * 2.0_f32.powi(exponent - 15),
    }
}

/// The scene texture deliberately has no COPY_SRC usage. Sample its exact texel into
/// a one-pixel float target, then copy eight bytes, without changing production usage/masks.
fn scene_pixel(renderer: &Renderer) -> [f32; 4] {
    let scene = renderer.targets.offscreen().expect("HDR scene target");
    assert_eq!(scene.color_format, wgpu::TextureFormat::Rgba16Float);
    let device = &renderer.device;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("HDR fixture texel readback"),
        source: wgpu::ShaderSource::Wgsl(
            r#"
@group(0) @binding(0) var scene: texture_2d<f32>;
@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let points = array<vec2<f32>, 3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(points[index],0.0,1.0);
}
@fragment fn fragment() -> @location(0) vec4<f32> {
    return textureLoad(scene,vec2<i32>(32,32),0);
}
"#
            .into(),
        ),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("HDR fixture texel readback"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("HDR fixture scene input"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&scene.color_view),
        }],
    });
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("HDR fixture single texel"),
        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("HDR fixture readback"),
        size: u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("HDR fixture readback"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
    );
    renderer.queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let bytes = slice.get_mapped_range().unwrap();
    let pixel = std::array::from_fn(|i| half(u16::from_le_bytes([bytes[i * 2], bytes[i * 2 + 1]])));
    drop(bytes);
    buffer.unmap();
    pixel
}

#[test]
#[ignore = "needs a GPU"]
fn original_material_alpha_survives_hdr_intermediate_while_presentation_stays_opaque() {
    let _gpu = test_support::serial();
    let mut renderer = renderer();
    let resolved = renderer.apply_graphics(&GraphicsSettings {
        post: PostSettings { tonemap: Tonemap::Aces, ..Default::default() },
        ..Default::default()
    });
    assert!(resolved.is_exact(), "HDR fixture settings must be supported");
    assert!(renderer.is_hdr(), "the selected GPU must expose the HDR intermediate");
    for model in [GlossyModel::Shadowed, GlossyModel::VertexLit] {
        for (index, blend) in [(0, BlendMode::Opaque), (2, BlendMode::Opaque), (21, BlendMode::AlphaTest)] {
            let fixture = Fixture::case(index, model);
            let scene = Scene::new(&mut renderer, &fixture, blend);
            let published = scene.capture(&mut renderer, &fixture);
            assert_eq!(published.pixel(32, 32).unwrap()[3], 255, "published captures remain opaque");
            let actual = scene_pixel(&renderer)[3];
            let expected = f32::from(data::ALPHA[index][0]) / 255.0;
            assert!(
                (actual - expected).abs() <= 2.0 / 255.0,
                "{model:?} case {index}: HDR alpha={actual}, expected {expected}"
            );
            scene.release(&mut renderer);
        }
    }
    let fixture = Fixture::case(0, GlossyModel::Custom);
    let scene = Scene::new(&mut renderer, &fixture, BlendMode::Opaque);
    scene.capture(&mut renderer, &fixture);
    assert_eq!(scene_pixel(&renderer)[3], 1.0, "Custom retains its existing opaque scene alpha");
    scene.release(&mut renderer);
}
