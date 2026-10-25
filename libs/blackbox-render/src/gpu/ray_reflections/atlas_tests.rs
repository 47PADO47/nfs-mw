//! Higher-detail road cache must preserve detail lost in the general atlas.
use super::*;
use blackbox_gpu_passes::test_support::{Gpu, serial};
#[test]
#[ignore = "requires a GPU"]
fn bounded_road_cache_preserves_high_frequency_texture_detail() {
    let _serial = serial();
    let gpu = Gpu::new(wgpu::Backends::VULKAN);
    let source = gpu.target(512);
    let mut pixels = Vec::with_capacity(512 * 512 * 4);
    for y in 0..512 {
        for x in 0..512 {
            let v = match (x / 2 + y / 2) % 2 {
                0 => 0,
                _ => 255,
            };
            pixels.extend_from_slice(&[v, v, v, 255]);
        }
    }
    gpu.queue.write_texture(
        source.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(2048), rows_per_image: Some(512) },
        source.size(),
    );
    let source = source.create_view(&Default::default());
    let shader=gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor{label:Some("atlas detail test"),source:wgpu::ShaderSource::Wgsl("@group(0) @binding(0) var source:texture_2d_array<f32>; @group(0) @binding(1) var linear_sampler:sampler; struct Out { @builtin(position) clip:vec4<f32>, @location(0) uv:vec2<f32> }; @vertex fn vs(@builtin(vertex_index) i:u32)->Out { let uv=vec2(f32((i<<1u)&2u),f32(i&2u)); return Out(vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),0.0,1.0),uv); } @fragment fn fs(in:Out)->@location(0) vec4<f32> {return textureSampleLevel(source,linear_sampler,in.uv,0,0.0);}".into())});
    let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("atlas detail test"),
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
            targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let mut variances = Vec::new();
    for mut atlas in [Atlas::new(&gpu.device), Atlas::detail(&gpu.device)] {
        assert_eq!(atlas.layer(&gpu.device, &gpu.queue, 1, &source), Some(0));
        let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas detail test"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&atlas.view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&atlas.sampler) },
            ],
        });
        let target = gpu.target(512);
        let view = target.create_view(&Default::default());
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("atlas detail test"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        let image = gpu.finish(encoder, &target);
        let values: Vec<f64> = image.as_chunks::<4>().0.iter().map(|p| f64::from(p[0])).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        variances.push(values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / values.len() as f64);
    }
    assert!(
        variances[0] < 64.0 && variances[1] > 1000.0,
        "road texture detail must survive the high-detail cache: {variances:?}"
    );
}
