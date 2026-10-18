//! Mixed-resolution routing, continuous handoffs and live resource replacement.
use super::{
    fit::Params,
    maps::Maps,
    sampling_tests::render_pipeline,
    tests::{dark_pixels, scene},
};
use crate::gpu::test_support;
use blackbox_gfx::{RenderBackend, RenderExperiments};
use bytemuck::Zeroable;
use glam::Mat4;

#[test]
#[ignore = "needs a GPU"]
fn mixed_resolution_maps_route_all_layers_and_blend_continuously() {
    let _serial = test_support::serial();
    let Some(r) = test_support::headless((4, 1)) else { return };
    let maps = Maps::new(&r.device, &r.shared.glossy.material, 8, 4, Some(16), None, None);
    let source = format!(
        "{}\n
        @vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32>{{
            let p=vec2<f32>(f32((i<<1u)&2u),f32(i&2u));return vec4<f32>(p*2.0-1.0,0.5,1.0);}}
        @fragment fn fs(@builtin(position) p:vec4<f32>)->@location(0) vec4<f32>{{
            let point=vec3<f32>(0.0,0.0,0.5);let n=vec3<f32>(0.0,0.0,1.0);
            return vec4<f32>(cascade_sample(point,n,u32(p.x)).x,sun_sample(point,n).x,0.0,1.0);}}",
        include_str!("../../shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "0")
    );
    let shader = r.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("mixed-resolution production receiver probe"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = render_pipeline(&r.device, &shader, Some(wgpu::TextureFormat::Rgba8Unorm), false);
    // The production material group is a superset of this probe's inferred layout.
    let group = r.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("mixed-resolution receiver inputs"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 1, resource: maps.buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&maps.view) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&maps.sampler) },
            wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&maps.detail) },
        ],
    });
    let output = r.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("cascade route and handoff pixels"),
        size: wgpu::Extent3d { width: 4, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = output.create_view(&Default::default());
    let mut encoder = r.device.create_command_encoder(&Default::default());
    for (layer, depth) in maps.layers.iter().zip([0.25, 0.75, 0.25, 0.75]) {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: layer,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(depth), store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
    }
    r.queue.submit([encoder.finish()]);
    let mut params = Params::zeroed();
    params.matrices = [Mat4::IDENTITY.to_cols_array(); 4];
    params.options = [1.0, 8.0, 1.5, 0.0];
    params.resolutions = [16.0, 16.0, 8.0, 8.0];
    params.texels = [1.0, 2.0, 4.0, 8.0];
    params.depth_spans = [10.0; 4];
    params.sun = [0.0, 0.0, 1.0, 0.0];
    params.softness = [0.01, 0.0, 1.0, 1.0];
    params.splits = [50.0, 134.0, 320.0, 960.0];
    params.camera[3] = 4.0;
    params.forward[2] = 1.0;
    for pcss in [false, true] {
        params.softness[1] = u8::from(pcss) as f32;
        let mut previous: Option<u8> = None;
        for step in 0..89 {
            let depth = 30.0 + step as f32 * 0.25;
            params.camera[2] = 0.5 - depth;
            r.queue.write_buffer(&maps.buffer, 0, bytemuck::bytes_of(&params));
            let mut encoder = r.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
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
            let pixels = r.read_raw_texture(&output, (4, 1), 4).unwrap();
            assert_eq!([pixels[0], pixels[4], pixels[8], pixels[12]], [0, 255, 0, 255], "map routing");
            let t = ((depth - 32.5) / 17.5).clamp(0.0, 1.0);
            let expected = (t * t * (3.0 - 2.0 * t) * 255.0).round() as u8;
            assert!(pixels[1].abs_diff(expected) <= 1, "overlap depth {depth}: {pixels:?}");
            if let Some(before) = previous {
                assert!(pixels[1].abs_diff(before) <= 6, "handoff discontinuity");
            }
            previous = Some(pixels[1]);
        }
    }
}

#[test]
#[ignore = "needs a GPU"]
fn wide_detail_rebinds_and_roundtrips_without_losing_contact() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let (frame, ground, casters) = scene(&mut r);
    r.set_shadow_casters(Some(&casters));
    let off = r.capture(240, 160, &frame, &ground).unwrap();
    let mut reference = None;
    for wide in [false, true, false, true] {
        r.set_render_experiments(RenderExperiments {
            sun_shadows: true,
            far_shadows: true,
            shadow_resolution: 512,
            shadow_detail_resolution: 1024,
            wide_shadow_detail: wide,
            shadow_distance: 320.0,
            ..Default::default()
        });
        let image = r.capture(240, 160, &frame, &ground).unwrap();
        assert!(dark_pixels(&off, &image) > 50, "contact shadow coverage");
        let expected_mib = match wide {
            true => 10,
            false => 4,
        };
        assert_eq!(r.experiment_stats().map_mib, expected_mib);
        if !wide {
            if let Some(before) = &reference {
                assert_eq!(&image, before, "reference roundtrip");
            }
            reference = Some(image.clone());
        }
        assert_eq!(image, r.capture(240, 160, &frame, &ground).unwrap(), "no frame history");
    }
}
