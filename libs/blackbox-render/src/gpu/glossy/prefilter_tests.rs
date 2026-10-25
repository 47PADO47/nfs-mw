//! GPU integration checks for sub-texel energy, cube edges, and the cost of dirty versus cached filtering.
use super::*;
use crate::gpu::test_support;
use glam::Vec3;

#[test]
fn source_pyramid_shader_validates() {
    let module = naga::front::wgsl::parse_str(source::SHADER).unwrap();
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty())
        .validate(&module)
        .unwrap();
}

fn samples(r: &crate::Renderer, filter: &Prefilter, points: &[[f32; 4]]) -> Vec<[f32; 4]> {
    let code = r#"
        @group(0) @binding(0) var encoded: texture_cube<f32>;
        @group(0) @binding(1) var linear: texture_cube<f32>;
        @group(0) @binding(2) var samp: sampler;
        @group(0) @binding(3) var<storage, read> points: array<vec4<f32>>;
        @group(0) @binding(4) var<storage, read_write> values: array<vec4<f32>>;
        @compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id: vec3<u32>) {
            let p = points[id.x];
            let lod = p.w * f32(textureNumLevels(encoded) - 1u);
            values[id.x] = vec4(textureSampleLevel(encoded,samp,p.xyz,lod).rgb,
                textureSampleLevel(linear,samp,p.xyz,lod).r);
        }"#;
    let shader = r.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("independent filter sampling"),
        source: wgpu::ShaderSource::Wgsl(code.into()),
    });
    let pipeline = r.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let input = r.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(points),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let bytes = points.len() as u64 * 16;
    let output = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let sampler = r.device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    });
    let group = r.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&filter.view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&filter.linear_view) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            wgpu::BindGroupEntry { binding: 3, resource: input.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 4, resource: output.as_entire_binding() },
        ],
    });
    let mut encoder = r.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(points.len() as u32, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &read, 0, bytes);
    r.queue.submit([encoder.finish()]);
    read.slice(..).map_async(wgpu::MapMode::Read, |v| v.unwrap());
    r.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    bytemuck::cast_slice(&read.slice(..).get_mapped_range().unwrap()).to_vec()
}

fn update(r: &mut crate::Renderer) {
    let mut encoder = r.device.create_command_encoder(&Default::default());
    let glossy = r.glossy.as_mut().unwrap();
    glossy.filtered.as_mut().unwrap().encode(&r.device, &mut encoder, &glossy.environment, &glossy.sampler);
    r.queue.submit([encoder.finish()]);
}

#[test]
#[ignore = "needs a GPU"]
fn subtexel_reflection_energy_does_not_crawl_between_checker_phases() {
    let _serial = test_support::serial();
    let mut r = test_support::headless((2, 2)).expect("GPU adapter");
    let points: Vec<_> = [0.0, 0.12, 0.35, 0.65, 1.0]
        .into_iter()
        .flat_map(|rough| {
            [
                [1.0, 0.1, 0.3, rough],
                [-1.0, -0.1, 0.9, rough],
                [0.1, 1.0, 0.2, rough],
                [0.3, 0.1, 1.0, rough],
                [1.0, 0.0, 1.0, rough],
            ]
        })
        .collect();
    let mut previous = None;
    for phase in 0..2 {
        let face: Vec<u8> = (0..512 * 512)
            .flat_map(|i| {
                let c = (((i % 512 + i / 512 + phase) & 1) * 255) as u8;
                [c, c, c, 255]
            })
            .collect();
        r.set_environment_faces(512, [&face; 6]);
        update(&mut r);
        let actual = samples(&r, r.glossy.as_ref().unwrap().filtered.as_ref().unwrap(), &points);
        for p in &actual {
            assert!(
                p.iter().all(|c| (*c - 0.5).abs() < 0.008),
                "both energy contracts preserve checker average: {p:?}"
            );
        }
        if let Some(before) = previous {
            assert_eq!(before, actual, "subpixel phase must not flicker");
        }
        previous = Some(actual);
    }
}

#[test]
#[ignore = "needs a GPU"]
fn a_continuous_environment_has_no_filtered_cube_edge_steps() {
    let _serial = test_support::serial();
    let mut r = test_support::headless((2, 2)).expect("GPU adapter");
    let faces: [Vec<u8>; 6] = std::array::from_fn(|face| {
        (0..64 * 64)
            .flat_map(|i| {
                let d = super::super::env::face_direction(
                    face,
                    (i % 64) as f32 / 64.0 + 0.5 / 64.0,
                    (i / 64) as f32 / 64.0 + 0.5 / 64.0,
                );
                let rgb = (d * 0.25 + Vec3::splat(0.5)).to_array().map(|v| (v * 255.0).round() as u8);
                [rgb[0], rgb[1], rgb[2], 255]
            })
            .collect()
    });
    r.set_environment_faces(64, faces.each_ref().map(Vec::as_slice));
    update(&mut r);
    for rough in [0.0, 0.2, 0.45, 0.8, 1.0] {
        let pairs = [
            [1.0, 0.0, 0.9999, rough],
            [0.9999, 0.0, 1.0, rough],
            [-1.0, 0.9999, 0.0, rough],
            [-0.9999, 1.0, 0.0, rough],
            [0.0, -1.0, -0.9999, rough],
            [0.0, -0.9999, -1.0, rough],
        ];
        let values = samples(&r, r.glossy.as_ref().unwrap().filtered.as_ref().unwrap(), &pairs);
        for pair in values.as_chunks::<2>().0 {
            for c in 0..4 {
                assert!((pair[0][c] - pair[1][c]).abs() < 0.006, "edge at roughness {rough}: {pair:?}");
            }
        }
    }
}

#[test]
#[ignore = "GPU timestamp diagnostic, no gameplay performance claim"]
fn reflection_filter_update_budget() {
    let _serial = test_support::serial();
    let mut r = test_support::headless((2, 2)).expect("GPU adapter");
    assert!(r.device.features().contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS));
    let face = [160u8, 120, 70, 255].repeat(1024 * 1024);
    r.set_environment_faces(1024, [&face; 6]);
    update(&mut r);
    let queries =
        r.device.create_query_set(&wgpu::QuerySetDescriptor { label: None, ty: wgpu::QueryType::Timestamp, count: 2 });
    let resolve = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    for dirty in [true, false] {
        let mut times = Vec::new();
        for _ in 0..24 {
            let mut encoder = r.device.create_command_encoder(&Default::default());
            encoder.write_timestamp(&queries, 0);
            let glossy = r.glossy.as_mut().unwrap();
            let filter = glossy.filtered.as_mut().unwrap();
            filter.dirty = dirty;
            filter.encode(&r.device, &mut encoder, &glossy.environment, &glossy.sampler);
            encoder.write_timestamp(&queries, 1);
            encoder.resolve_query_set(&queries, 0..2, &resolve, 0);
            encoder.copy_buffer_to_buffer(&resolve, 0, &read, 0, 16);
            r.queue.submit([encoder.finish()]);
            read.slice(..).map_async(wgpu::MapMode::Read, |v| v.unwrap());
            r.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let data = read.slice(..).get_mapped_range().unwrap();
            let ticks: &[u64] = bytemuck::cast_slice(&data);
            times.push((ticks[1] - ticks[0]) as f64 * f64::from(r.queue.get_timestamp_period()) / 1e6);
            drop(data);
            read.unmap();
        }
        times.sort_by(f64::total_cmp);
        eprintln!("reflection filter dirty={dirty}: median {:.4} ms, p95 {:.4} ms", times[12], times[22]);
    }
}

#[test]
#[ignore = "GPU capture diagnostic, synthetic geometry only"]
fn reflection_capture_budget() {
    use blackbox_gfx::*;
    use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
    use glam::Mat4;
    let _serial = test_support::serial();
    let mut r = test_support::headless((1280, 720)).expect("GPU adapter");
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Prelit);
    for x in -8..=8 {
        for y in -8..=8 {
            let p = Vec3::new(x as f32 * 8.0, y as f32 * 8.0, -4.0);
            mesh.cuboid(p, p + Vec3::new(3.0, 3.0, 6.0), prelit([0.3, 0.5, 0.7], 1.0));
        }
    }
    let scenery = [Instance::new(mesh.finish().upload(&mut r, "capture cost fixture"), Mat4::IDENTITY)];
    r.create_glossy_material(&GlossyMaterial {
        surface: Some(GlossySurface { roughness: 0.4, ..Default::default() }),
        ..Default::default()
    });
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, -2.0, 2.0), Vec3::Y, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1280.0 / 720.0, near: 0.1 },
        camera_position: Vec3::new(0.0, -2.0, 2.0),
        light_dir: Vec3::NEG_Z,
        clear_color: [0.1; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    for (label, faces) in [("all six", 63u8), ("one per frame", 1), ("cached", 0)] {
        let mut times = Vec::new();
        for index in 0..24 {
            let refresh = match faces {
                1 => 1 << (index % 6),
                _ => faces,
            };
            r.set_reflection_scene(Some(ReflectionScene {
                sun_shadows: false,
                fade_distance: None,
                origin: Vec3::ZERO,
                frame,
                faces: [&scenery; 6],
                quality: ReflectionQuality::High,
                refresh,
            }));
            r.render(&frame, &[]).unwrap();
            r.read_output().unwrap().unwrap();
            if index >= 8 {
                times.push(r.timing.as_ref().unwrap().latest.unwrap()[0]);
            }
        }
        times.sort_by(f32::total_cmp);
        eprintln!("default High cube {label}: GPU frame median {:.4} ms, p95 {:.4} ms", times[8], times[15]);
    }
}
