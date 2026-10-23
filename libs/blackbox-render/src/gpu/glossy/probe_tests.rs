//! A finite room's depth must move reflections to their receiver position on all six axes.
use crate::gpu::test_support;
use glam::Vec3;
use wgpu::util::DeviceExt;

#[test]
#[ignore = "needs a GPU"]
fn local_probe_reprojects_six_room_planes_and_preserves_sky_rough_and_disabled() {
    let _serial = test_support::serial();
    let r = test_support::headless((1, 1)).expect("GPU adapter");
    let depths = r.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("six room planes"),
        size: wgpu::Extent3d { width: 8, height: 8, depth_or_array_layers: 6 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let array = depths.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let output = r.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("probe direction result"),
        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target = output.create_view(&Default::default());
    let controls = r.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("probe test inputs"),
        contents: bytemuck::cast_slice(&[0.0_f32; 12]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let origin = r.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("probe test origin"),
        contents: bytemuck::cast_slice(&[0.0_f32, 0.0, 0.0, 1.0]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let source = format!("{}
{}", include_str!("../../shaders/reflection_probe.wgsl").replace("@group(2)", "@group(0)"),
        "struct Inputs { world: vec4<f32>, ray: vec4<f32>, unused: vec4<f32>, };
@group(0) @binding(12) var<uniform> inputs: Inputs;
        @vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32> {
            let p=vec2<f32>(f32((i<<1u)&2u),f32(i&2u));return vec4<f32>(p*2.0-1.0,0.0,1.0); }
        @fragment fn fs()->@location(0) vec4<f32> {return vec4<f32>(probe_direction(inputs.world.xyz, inputs.ray.xyz, inputs.world.w),1.0);}");
    let shader = r.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production probe test"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = r.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("probe test"),
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
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba32Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(),
        multisample: Default::default(),
        depth_stencil: None,
        multiview_mask: None,
        cache: None,
    });
    let group = r.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("probe test bindings"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 10, resource: wgpu::BindingResource::TextureView(&array) },
            wgpu::BindGroupEntry { binding: 11, resource: origin.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 12, resource: controls.as_entire_binding() },
        ],
    });
    for (depth, roughness, enabled) in [(0.05, 0.1, true), (0.0, 0.1, true), (0.05, 0.8, true), (0.05, 0.1, false)] {
        for ray in [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z] {
            let offset = match ray.z.abs() > 0.5 {
                true => Vec3::X,
                false => Vec3::Z,
            };
            r.queue.write_buffer(
                &controls,
                0,
                bytemuck::cast_slice(&[offset.extend(roughness).to_array(), ray.extend(0.0).to_array(), [0.0; 4]]),
            );
            r.queue.write_buffer(&origin, 0, bytemuck::cast_slice(&[0.0, 0.0, 0.0, f32::from(enabled)]));
            let mut encoder = r.device.create_command_encoder(&Default::default());
            for face in 0..6 {
                let view = depths.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: face,
                    array_layer_count: Some(1),
                    ..Default::default()
                });
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(depth),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                });
            }
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target,
                        depth_slice: None,
                        resolve_target: None,
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
            r.queue.submit([encoder.finish()]);
            let bytes = r.read_raw_texture(&output, (1, 1), 16).unwrap();
            let xyz: Vec<_> = bytes.as_chunks::<4>().0.iter().take(3).map(|v| f32::from_le_bytes(*v)).collect();
            let actual = Vec3::new(xyz[0], xyz[1], xyz[2]);
            let expected = match depth > 0.0 && roughness < 0.6 && enabled {
                true => (ray * 5.0 + offset).normalize(),
                false => ray,
            };
            assert!(actual.abs_diff_eq(expected, 0.001), "{ray:?} -> {actual:?}, expected {expected:?}");
        }
    }
}
