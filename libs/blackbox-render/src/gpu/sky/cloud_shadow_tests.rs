//! GPU checks for a density-derived, deterministic transmission field.
use super::*;
use blackbox_gfx::RenderBackend;

fn map(renderer: &mut Renderer, state: SkyState) -> Vec<u8> {
    renderer.set_sky_state(state);
    let globals = renderer.scene_globals(&frame(), None, false, false);
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    let sky = renderer.sky.as_mut().unwrap();
    sky.prepare(&renderer.device, &renderer.queue, renderer.targets.color_format(), None);
    sky.prepare_cloud_shadow(&renderer.device, &renderer.queue, &mut encoder, &renderer.shared.shadows.cloud, globals);
    let readback = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cloud shadow readback"),
        size: 256 * 256 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: renderer.shared.shadows.cloud.texture(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(1024), rows_per_image: Some(256) },
        },
        wgpu::Extent3d { width: 256, height: 256, depth_or_array_layers: 1 },
    );
    renderer.queue.submit([encoder.finish()]);
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    renderer.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let result = readback.slice(..).get_mapped_range().unwrap().to_vec();
    readback.unmap();
    result
}

#[test]
#[ignore = "needs a GPU"]
fn cloud_shadow_map_is_neutral_when_clear_and_changes_with_wind() {
    let _gpu = test_support::serial();
    let mut renderer = test_support::headless((32, 32)).expect("selected GPU must be available");
    let source = texture(&mut renderer, "cloud shadow fixture", 1, &[255; 4]);
    renderer
        .create_sky_material(&SkyMaterialDesc { kind: SkyLayerKind::Base, diffuse: source, miscellaneous: source })
        .unwrap();
    let clear = SkyState { enhanced: true, ..Default::default() };
    assert!(map(&mut renderer, clear).as_chunks::<4>().0.iter().all(|p| p[0] == 255));
    let cloudy = SkyState { cloud_coverage: 1.0, ..clear };
    let a = map(&mut renderer, cloudy);
    assert!(a.as_chunks::<4>().0.iter().any(|p| p[0] < 240), "clouds must attenuate sunlight");
    assert_eq!(a, map(&mut renderer, cloudy), "fixed cloud state is deterministic");
    let moved = map(&mut renderer, SkyState { scroll: 80.0, ..cloudy });
    assert!(
        a.as_chunks::<4>().0.iter().zip(moved.as_chunks::<4>().0.iter()).filter(|(a, b)| a[0] != b[0]).count() > 100,
        "wind must change the field"
    );
}

#[test]
#[ignore = "needs a GPU"]
fn cloud_shadows_change_world_direct_light_without_geometry_shadows() {
    let _gpu = test_support::serial();
    let mut renderer = test_support::headless((64, 64)).expect("selected GPU must be available");
    let source = texture(&mut renderer, "white", 1, &[255; 4]);
    renderer
        .create_sky_material(&SkyMaterialDesc { kind: SkyLayerKind::Base, diffuse: source, miscellaneous: source })
        .unwrap();
    let ground = texture(&mut renderer, "ground", 1, &[64, 64, 64, 255]);
    let state = SkyState { enhanced: true, cloud_coverage: 1.0, ..Default::default() };
    let field = map(&mut renderer, state);
    let (index, _) = field.as_chunks::<4>().0.iter().enumerate().min_by_key(|(_, p)| p[0]).unwrap();
    let x = (index % 256) as f32 * 8000.0 / 256.0 - 4000.0;
    let y = (index / 256) as f32 * 8000.0 / 256.0 - 4000.0;
    let center = Vec3::new(x, y, 0.0);
    let vertices = [[-500.0, -500.0], [-500.0, 500.0], [500.0, 500.0], [500.0, -500.0]].map(|p| Vertex {
        position: [p[0], p[1], 0.0],
        normal: [0.0, 0.0, 1.0],
        color_bgra: [255; 4],
        uv: [0.5; 2],
    });
    let mesh = renderer.create_mesh(&MeshDesc {
        label: "cloud receiver",
        vertices: &vertices,
        secondary_uvs: None,
        tangents: None,
        indices: &[0, 1, 2, 0, 2, 3],
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: Some(ground),
            blend: BlendMode::Opaque,
            shading: Shading::World,
            two_sided: true,
        }],
    });
    let mut view = frame();
    view.camera_position = center + Vec3::Z * 400.0;
    view.view = glam::camera::rh::view::look_at_mat4(view.camera_position, center, Vec3::Y);
    view.projection = Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 };
    let instance = Instance::new(mesh, Mat4::from_translation(center));
    let settings =
        blackbox_gfx::RenderExperiments { original_world_materials: true, sun_shadows: false, ..Default::default() };
    renderer.set_render_experiments(settings);
    renderer.render(&view, &[instance]).unwrap();
    let off = renderer.read_output().unwrap().unwrap().pixel(32, 32).unwrap();
    renderer.set_render_experiments(blackbox_gfx::RenderExperiments { cloud_shadows: true, ..settings });
    renderer.render(&view, &[instance]).unwrap();
    let on = renderer.read_output().unwrap().unwrap().pixel(32, 32).unwrap();
    assert!((on[0] as u16) + 5 < off[0] as u16, "clouds should shade world with object shadows off: {off:?} -> {on:?}");
    renderer.set_render_experiments(settings);
    renderer.render(&view, &[instance]).unwrap();
    assert_eq!(renderer.read_output().unwrap().unwrap().pixel(32, 32).unwrap(), off, "off restores baseline");
}

#[test]
fn camera_snap_moves_in_whole_map_texels() {
    let texel = 8000.0 / 256.0;
    let before = super::super::cloud_shadow::anchor(100.0);
    for camera in [-1030.0_f32, 100.1, 140.0, 4200.0] {
        let displacement = (super::super::cloud_shadow::anchor(camera) - before) / texel;
        assert_eq!(displacement, displacement.round());
    }
}
