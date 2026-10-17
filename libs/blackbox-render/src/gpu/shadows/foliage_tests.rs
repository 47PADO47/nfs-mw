//! Authored leaf normals describe lighting, not the cutout card's contact plane.
use crate::gpu::test_support;
use blackbox_gfx::{
    AoQuality, AoSettings, BlendMode, FrameParams, GraphicsSettings, Instance, LightingDebug, PixelFormat, Projection,
    RenderBackend, RenderExperiments, Shading, SurfaceLight, TextureDesc,
};
use blackbox_gfx_testkit::mesh::MeshBuilder;
use glam::{Mat4, Vec3};

#[test]
#[ignore = "requires a real GPU"]
fn foliage_contact_plane_does_not_create_self_shadows_during_camera_motion() {
    let _serial = test_support::serial();
    let mut r = test_support::headless((240, 160)).expect("foliage GPU");
    let pixels: Vec<u8> = (0..16 * 16)
        .flat_map(|i| {
            [
                64,
                128,
                32,
                match i % 4 {
                    0 => 0,
                    _ => 255,
                },
            ]
        })
        .collect();
    let texture = r.create_texture(&TextureDesc {
        label: "leaf cutout",
        width: 16,
        height: 16,
        format: PixelFormat::Rgba8,
        mips: vec![&pixels],
    });
    let mut mesh = MeshBuilder::new();
    mesh.state(Some(texture), BlendMode::AlphaTest, Shading::Tree);
    mesh.flat_quad(
        [Vec3::new(-8.0, -8.0, -6.0), Vec3::new(8.0, -8.0, 6.0), Vec3::new(8.0, 8.0, 6.0), Vec3::new(-8.0, 8.0, -6.0)],
        [255; 4],
    );
    let mut mesh = mesh.finish();
    for vertex in &mut mesh.vertices {
        vertex.normal = Vec3::Z.to_array();
    }
    let mesh = mesh.upload(&mut r, "leaf card with authored lighting normals");
    let instances = [Instance::new(mesh, Mat4::IDENTITY)];
    for offset in [-0.03, 0.0, 0.03] {
        let eye = Vec3::new(-3.0 + offset, -10.0, 10.0);
        let frame = FrameParams {
            view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
            projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.5, near: 0.1 },
            camera_position: eye,
            light_dir: -Vec3::new(0.8, 0.0, 0.6),
            clear_color: [0.0; 3],
            fog: None,
            scattering: None,
            surface_light: Some(SurfaceLight {
                ambient: [0.3; 3],
                ambient_alpha: 1.0,
                diffuse: [0.7; 3],
                specular: [0.0; 3],
                specular_power: 1.0,
                reflection_scale: 0.0,
                reflective_specular_scale: 1.0,
                shadow_strength: 1.0,
            }),
            camera_cut: false,
        };
        let settings = RenderExperiments {
            original_world_materials: true,
            lighting_debug: LightingDebug::SunVisibility,
            contact_length: 0.63,
            ..Default::default()
        };
        r.set_render_experiments(settings);
        let baseline = r.capture(240, 160, &frame, &instances).unwrap();
        r.set_render_experiments(RenderExperiments { contact_shadows: true, ..settings });
        let contact = r.capture(240, 160, &frame, &instances).unwrap();
        let visible: Vec<_> =
            baseline.as_chunks::<4>().0.iter().zip(contact.as_chunks::<4>().0).filter(|(a, _)| a[0] > 250).collect();
        assert!(visible.len() > 3000);
        let dark = visible.iter().filter(|(_, b)| b[0] < 245).count();
        assert!(dark * 100 < visible.len(), "false leaf contacts {dark}/{} at offset {offset}", visible.len());
        r.apply_graphics(&GraphicsSettings {
            ao: AoSettings { quality: AoQuality::High, intensity: 1.0, ..Default::default() },
            ..Default::default()
        });
        r.set_render_experiments(RenderExperiments { lighting_debug: LightingDebug::AmbientVisibility, ..settings });
        let ambient = r.capture(240, 160, &frame, &instances).unwrap();
        let visible: Vec<_> =
            baseline.as_chunks::<4>().0.iter().zip(ambient.as_chunks::<4>().0).filter(|(a, _)| a[0] > 250).collect();
        let dark = visible.iter().filter(|(_, b)| b[0] < 245).count();
        assert!(dark * 100 < visible.len(), "false leaf AO {dark}/{} at offset {offset}", visible.len());
        r.apply_graphics(&GraphicsSettings::default());
    }
}
