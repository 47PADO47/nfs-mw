//! A directional-light filter must not change when depth-map and receiver origins move together.
use super::fit::Params;
use crate::gpu::test_support;
use bytemuck::Zeroable;
use glam::{Mat4, Vec3};

#[test]
#[ignore = "needs a GPU"]
fn blocker_search_is_invariant_to_light_depth_origin() {
    let _serial = test_support::serial();
    let Some(r) = test_support::headless((1, 1)) else { return };
    let map = r.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("translated shadow-depth fixture"),
        size: wgpu::Extent3d { width: 32, height: 32, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth = map.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2),
        ..Default::default()
    });
    let array = map.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let output = r.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shadow sample result"),
        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let output_view = output.create_view(&Default::default());
    let vertex = "@vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32>{
        let p=vec2<f32>(f32((i<<1u)&2u),f32(i&2u));return vec4<f32>(p*2.0-1.0,0.5,1.0);}";
    let source = format!(
        "{}\n{}\n@fragment fn fs()->@location(0) vec4<f32>{{
        return vec4<f32>(cascade_sample(vec3<f32>(0.0),vec3<f32>(0.0,0.0,1.0),0u),1.0);}}",
        include_str!("../../shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "0"),
        vertex
    );
    let shader = r.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production shadow sampling probe"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = render_pipeline(&r.device, &shader, Some(wgpu::TextureFormat::Rgba8Unorm), false);
    let uniform = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("shadow probe params"),
        size: std::mem::size_of::<Params>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let sampler = r.device.create_sampler(&wgpu::SamplerDescriptor {
        compare: Some(wgpu::CompareFunction::LessEqual),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let group = r.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("shadow probe inputs"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 1, resource: uniform.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&array) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&sampler) },
            wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&array) },
        ],
    });
    let mut images = Vec::new();
    for origin in [400.0_f32, 900.0] {
        let mut params = Params::zeroed();
        params.matrices[0] = Mat4::from_translation(Vec3::new(0.0, 0.0, origin / 1000.0)).to_cols_array();
        params.options = [1.0, 32.0, 0.25, 0.0];
        params.sun = [0.0, 0.0, 1.0, 0.0];
        params.texels = [1.0; 4];
        params.depth_spans = [1000.0; 4];
        params.softness = [0.27_f32.to_radians().tan(), 1.0, 0.0, 0.0];
        r.queue.write_buffer(&uniform, 0, bytemuck::bytes_of(&params));
        // The central four-texel blocker stays 300 units in front of the receiver;
        // surrounding geometry stays 100 units in front. Changing the search area
        // therefore changes the estimated penumbra, not just the blocker count.
        let source = format!(
            "{vertex}\n@fragment fn fs(@builtin(position) p:vec4<f32>)->@builtin(frag_depth) f32{{
            if p.x<14.0 || p.x>=18.0 || p.y<14.0 || p.y>=18.0 {{return {};}} return {};}}",
            (origin - 100.0) / 1000.0,
            (origin - 300.0) / 1000.0
        );
        let shader = r.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("translated depth caster"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let writer = render_pipeline(&r.device, &shader, None, true);
        let mut encoder = r.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&writer);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
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
        images.push(r.read_raw_texture(&output, (1, 1), 4).unwrap());
    }
    assert!(
        images[0][1] > 0 && images[0][2] > 12,
        "probe must find blockers and exercise contact hardening: {images:?}"
    );
    assert_eq!(
        images[0], images[1],
        "changing only the normalized depth origin must not move blocker taps or change penumbra"
    );
}

pub(super) fn render_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    color: Option<wgpu::TextureFormat>,
    depth: bool,
) -> wgpu::RenderPipeline {
    let targets =
        color.map(|format| wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("shadow probe"),
        layout: None,
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: match color {
                Some(_) => std::slice::from_ref(&targets),
                None => &[],
            },
        }),
        depth_stencil: depth.then_some(wgpu::DepthStencilState {
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
    })
}

#[test]
#[ignore = "needs a GPU"]
fn dense_wide_penumbra_adds_taps_and_switches_continuously() {
    let _serial = test_support::serial();
    let Some(r) = test_support::headless((1, 1)) else { return };
    let texture = |size, format, usage| {
        r.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wide penumbra fixture"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let map = texture(
        32,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let depth = map.create_view(&Default::default());
    let array = map.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let output = texture(
        256,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let view = output.create_view(&Default::default());
    let vertex = "@vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32>{
        let p=vec2<f32>(f32((i<<1u)&2u),f32(i&2u));return vec4<f32>(p*2.0-1.0,0.5,1.0);}";
    let shader = |source: String| {
        r.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wide penumbra probe"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        })
    };
    let writer = shader(format!(
        "{vertex}\n@fragment fn fs(@builtin(position) p:vec4<f32>)->@builtin(frag_depth) f32{{
        if p.x>=16.0 && p.x<17.0 && p.y>=16.0 && p.y<17.0 {{return 0.4;}}return 1.0;}}"
    ));
    let writer = render_pipeline(&r.device, &writer, None, true);
    let source = format!(
        "{}\n{vertex}\n@fragment fn fs(@builtin(position) p:vec4<f32>)->@location(0) vec4<f32>{{
        let world=vec3<f32>(p.x/128.0-1.0,1.0-p.y/128.0,0.8);
        let v=cascade_sample(world,vec3<f32>(0.0,0.0,1.0),0u).x;return vec4<f32>(v,v,v,1.0);}}",
        include_str!("../../shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "0")
    );
    let pipeline = render_pipeline(&r.device, &shader(source), Some(wgpu::TextureFormat::Rgba8Unorm), false);
    let uniform = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("penumbra probe params"),
        size: std::mem::size_of::<Params>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let sampler = r.device.create_sampler(&wgpu::SamplerDescriptor {
        compare: Some(wgpu::CompareFunction::LessEqual),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let group = r.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("penumbra probe inputs"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 1, resource: uniform.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&array) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&sampler) },
            wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&array) },
        ],
    });
    let mut images = Vec::new();
    for (radius, dense) in [
        (8.0, false),
        (8.0, true),
        (8.0, false),
        (0.75, false),
        (0.75, true),
        (1.999, true),
        (2.001, true),
        (2.999, true),
        (3.001, true),
    ] {
        let mut params = Params::zeroed();
        params.matrices[0] = Mat4::IDENTITY.to_cols_array();
        params.options = [1.0, 32.0, radius, 0.0];
        params.sun = [0.0, 0.0, 1.0, 0.0];
        params.texels = [1.0; 4];
        params.depth_spans = [1000.0; 4];
        params.softness[2] = f32::from(u8::from(dense));
        r.queue.write_buffer(&uniform, 0, bytemuck::bytes_of(&params));
        let mut encoder = r.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&writer);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
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
        images.push(r.read_raw_texture(&output, (256, 256), 4).unwrap());
    }
    // The disk filter is a fixed-tap pattern here (no pixel noise in this probe), so a one-texel
    // blocker leaves unfiltered gaps. Real pixels rotate the pattern; dense mode only adds taps.
    let shadowed = |image: &[u8]| image.as_chunks::<4>().0.iter().filter(|p| p[0] < 255).count();
    eprintln!("Wide penumbra shadowed pixels: narrow {}, dense {}", shadowed(&images[0]), shadowed(&images[1]));
    assert!(shadowed(&images[0]) > 0 && shadowed(&images[1]) > 0, "both filters must see the blocker");
    assert_ne!(images[0], images[1], "dense mode must change a wide penumbra");
    assert_eq!(images[0], images[2], "filter toggle must restore the reference exactly");
    assert_eq!(images[3], images[4], "narrow filters remain identical");
    for (a, b) in [(5, 6), (7, 8)] {
        let jump = images[a].iter().zip(&images[b]).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
        assert!(jump <= 2, "filter handoff must be continuous ({jump}/255)");
    }
    if let Ok(directory) = std::env::var("BLACKBOX_SHADOW_FILTER_CAPTURE_DIR") {
        for (name, index) in [("sparse", 0), ("dense", 1)] {
            std::fs::write(std::path::Path::new(&directory).join(format!("penumbra-{name}.rgba")), &images[index])
                .unwrap();
        }
    }
}
