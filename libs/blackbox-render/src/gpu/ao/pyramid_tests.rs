//! Filtering must retain odd image edges, and toggling the hierarchy must release its resources.
use super::{Params, depth::DepthPyramid, pipelines::Pipelines, tests::scene_shading};
use crate::gpu::test_support;
use blackbox_gfx::{AoMethod, AoQuality, AoSettings, GraphicsSettings, RenderBackend, Shading};
use glam::Mat4;

#[test]
#[ignore = "needs a GPU"]
fn hierarchy_switches_restore_exact_frames_and_release_full_size_images() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless((319, 237)) else { return };
    for (size, method) in
        [(319, 237), (320, 240), (1, 1)].into_iter().flat_map(|size| AoMethod::ALL.map(|method| (size, method)))
    {
        eprintln!("hierarchy live switch size={size:?}, method={method}");
        let (mut frame, instances) = scene_shading(&mut r, size, true, Shading::Prelit);
        // Compare cold estimators when switching depth sources. Temporal convergence
        // is covered separately; evolving sample phases are not repeatable frames.
        frame.camera_cut = true;
        let settings = GraphicsSettings {
            ao: AoSettings { method, quality: AoQuality::Medium, depth_mips: true, ..Default::default() },
            ..Default::default()
        };
        r.apply_graphics(&settings);
        let filtered = r.capture(size.0, size.1, &frame, &instances).unwrap();
        assert_eq!(r.ao.as_ref().unwrap().depth.size, size);
        r.apply_graphics(&GraphicsSettings { ao: AoSettings { depth_mips: false, ..settings.ao }, ..settings });
        let direct = r.capture(size.0, size.1, &frame, &instances).unwrap();
        assert_eq!(r.ao.as_ref().unwrap().depth.size, (1, 1));
        assert_eq!(direct, r.capture(size.0, size.1, &frame, &instances).unwrap());
        r.apply_graphics(&settings);
        assert_eq!(filtered, r.capture(size.0, size.1, &frame, &instances).unwrap());
        r.apply_graphics(&GraphicsSettings::default());
        r.capture(size.0, size.1, &frame, &instances).unwrap();
        assert!(r.ao.is_none());
    }
}

#[test]
#[ignore = "needs a GPU"]
fn odd_depth_reduction_keeps_last_column_and_rejects_foreground_depth_blending() {
    let _serial = test_support::serial();
    let Some(r) = test_support::headless((5, 3)) else { return };
    let depth = r
        .device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("odd depth fixture"),
            size: wgpu::Extent3d { width: 5, height: 3, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default());
    let shader = r.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("odd depth fixture"),
        source: wgpu::ShaderSource::Wgsl(
            "@vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32>{
                let p=vec2<f32>(f32((i<<1u)&2u),f32(i&2u));return vec4<f32>(p*2.0-1.0,0.5,1.0);}
             @fragment fn fs(@builtin(position) p:vec4<f32>)->@builtin(frag_depth) f32{
                if p.x>4.0{return 0.1/20.0;}return 0.1/2.0;}"
                .into(),
        ),
    });
    let pipeline = r.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("odd depth fixture"),
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
    let params = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("depth reduction params"),
        size: std::mem::size_of::<Params>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    r.queue.write_buffer(
        &params,
        0,
        bytemuck::bytes_of(&Params {
            sizes: [5.0, 3.0, 5.0, 3.0],
            projection: [1.0, 1.0, 0.1, 1.0],
            effect: [1.25, 1.0, 40.0, 100.0],
            sampling: [1, 4, 6, 0],
            fog: [0.0; 4],
            view: Mat4::IDENTITY.to_cols_array(),
        }),
    );
    let mut encoder = r.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("odd depth fixture"),
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
    let mut pyramid = DepthPyramid::new(&r.device, (5, 3));
    let fallback = DepthPyramid::new(&r.device, (1, 1));
    pyramid.encode(&r.device, &mut encoder, &Pipelines::new(&r.device, None), &params, &depth, &fallback.view);
    r.queue.submit([encoder.finish()]);
    let bytes = r.read_raw_texture(pyramid.view.texture(), (2, 1), 4).unwrap();
    let values: Vec<_> = bytes.as_chunks::<4>().0.iter().map(|&p| f32::from_le_bytes(p)).collect();
    assert!((values[0] - 2.0).abs() < 0.0001, "flat depth changed: {values:?}");
    assert!(
        (values[1] - 20.0).abs() < 0.0001,
        "last odd column lost, or a foreground/background gap was averaged: {values:?}"
    );
}
