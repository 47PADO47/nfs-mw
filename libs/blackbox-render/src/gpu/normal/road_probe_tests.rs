//! Near probe occluders must not turn into enlarged distant puddle reflections.
use crate::gpu::test_support;
use glam::Vec3;
use wgpu::util::DeviceExt;

#[test]
#[ignore = "needs a GPU"]
fn road_probe_rejects_near_occluders_but_preserves_distant_and_static_environments() {
    let _serial = test_support::serial();
    let r = test_support::headless((1, 1)).expect("GPU adapter");
    let cube = |colour: [u8; 4]| {
        let texture = r.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("probe test cube"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 6 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        r.queue.write_texture(
            texture.as_image_copy(),
            &colour.repeat(6),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
            texture.size(),
        );
        texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        })
    };
    let red = cube([255, 0, 0, 255]);
    let sky = r.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sky fallback colours"),
        contents: bytemuck::cast_slice(&[[0.0_f32, 0.0, 1.0, 1.0]; 6]),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let depth = r.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("probe test occluder depths"),
        size: wgpu::Extent3d { width: 8, height: 8, depth_or_array_layers: 6 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let origin = r.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("probe origin"),
        contents: bytemuck::cast_slice(&[0.0_f32; 4]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let input = r.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("road receiver and ray"),
        contents: bytemuck::cast_slice(&[0.0_f32; 8]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let output = r.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("probe result"),
        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let shader = r.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production road probe regression"),
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "{}\n{}",
                include_str!("../../shaders/road_probe.wgsl").replace("@group(1)", "@group(0)"),
                "@group(0) @binding(10) var road_environment: texture_cube<f32>;
@group(0) @binding(11) var road_sampler: sampler;
@group(0) @binding(12) var road_filtered_environment: texture_cube<f32>;
struct Input { world: vec4<f32>, ray: vec4<f32>, }
@group(0) @binding(16) var<uniform> input: Input;
@vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32> {
    let p=vec2<f32>(f32((i<<1u)&2u),f32(i&2u));return vec4<f32>(p*2.0-1.0,0.0,1.0); }
@fragment fn fs()->@location(0) vec4<f32> {return vec4(road_probe_at(input.world.xyz,input.ray.xyz,0.07),1.0);}"
            )
            .into(),
        ),
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
                format: output.format(),
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
    let sampler = r.device.create_sampler(&Default::default());
    let group = r.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("road probe test"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 10, resource: wgpu::BindingResource::TextureView(&red) },
            wgpu::BindGroupEntry { binding: 11, resource: wgpu::BindingResource::Sampler(&sampler) },
            wgpu::BindGroupEntry { binding: 12, resource: wgpu::BindingResource::TextureView(&red) },
            wgpu::BindGroupEntry { binding: 13, resource: wgpu::BindingResource::TextureView(&depth_view) },
            wgpu::BindGroupEntry { binding: 14, resource: origin.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 15, resource: sky.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 16, resource: input.as_entire_binding() },
        ],
    });
    let probe = Vec3::new(1400.0, -2300.0, 150.0);
    for (z, enabled, displacement, expected) in [
        (0.25, 1.0, 8.0, Vec3::Z),                   // a nearby prop must not cover distant water
        (0.25, -1.0, 8.0, Vec3::Z),                  // independent of the car correction toggle
        (0.0025, 1.0, 8.0, Vec3::X),                 // distant scenery remains
        (0.0, 1.0, 8.0, Vec3::X),                    // sky has no parallax
        (0.25, 0.0, 8.0, Vec3::X),                   // uploaded static environments have no depth
        (0.25, 1.0, 0.0, Vec3::X),                   // a receiver at the probe needs no correction
        (0.25, 1.0, 0.25, Vec3::new(0.5, 0.0, 0.5)), // continuous transition
    ] {
        for ray in [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z] {
            r.queue.write_buffer(&origin, 0, bytemuck::cast_slice(&probe.extend(enabled).to_array()));
            r.queue.write_buffer(
                &input,
                0,
                bytemuck::cast_slice(&[
                    (probe + Vec3::Z * displacement).extend(0.0).to_array(),
                    ray.extend(0.0).to_array(),
                ]),
            );
            let mut encoder = r.device.create_command_encoder(&Default::default());
            for face in 0..6 {
                let view = depth.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: face,
                    array_layer_count: Some(1),
                    ..Default::default()
                });
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &view,
                        depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(z), store: wgpu::StoreOp::Store }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                });
            }
            let target = output.create_view(&Default::default());
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
            let values: Vec<_> = bytes.as_chunks::<4>().0.iter().map(|v| f32::from_le_bytes(*v)).collect();
            let actual = Vec3::new(values[0], values[1], values[2]);
            assert!(
                actual.abs_diff_eq(expected, 0.002),
                "depth {z}, enabled {enabled}, ray {ray:?}: {actual:?} != {expected:?}"
            );
        }
    }
}
