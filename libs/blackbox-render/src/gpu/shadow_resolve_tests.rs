//! Cascaded visibility history resource switches, camera cuts and disappearing casters.
use crate::gpu::test_support;
use blackbox_gfx::*;
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};
#[test]
#[ignore = "needs a GPU"]
fn cascaded_history_switches_and_cuts_preserve_visibility_without_trails() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless((240, 160)) else { return };
    let mut m = MeshBuilder::new();
    m.state(None, BlendMode::Opaque, Shading::Prelit);
    m.flat_quad(
        [
            Vec3::new(-10.0, -10.0, 0.0),
            Vec3::new(10.0, -10.0, 0.0),
            Vec3::new(10.0, 10.0, 0.0),
            Vec3::new(-10.0, 10.0, 0.0),
        ],
        prelit([0.7; 3], 1.0),
    );
    let ground = Instance::new(m.finish().upload(&mut r, "history receiver"), Mat4::IDENTITY);
    let mut m = MeshBuilder::new();
    m.state(None, BlendMode::Opaque, Shading::Prelit);
    m.cuboid(Vec3::new(-1.0, -2.0, 0.2), Vec3::new(1.0, 2.0, 1.5), prelit([0.5; 3], 1.0));
    let caster = Instance::new(m.finish().upload(&mut r, "history blocker"), Mat4::IDENTITY);
    let eye = Vec3::new(3.0, -7.0, 4.0);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.5, near: 0.1 },
        camera_position: eye,
        light_dir: Vec3::new(-0.3, 0.0, -1.0),
        clear_color: [1.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let options = RenderExperiments {
        sun_shadows: true,
        cascade_temporal: true,
        contact_shadows: true,
        shadow_resolution: 512,
        shadow_distance: 40.0,
        lighting_debug: LightingDebug::SunVisibility,
        ..Default::default()
    };
    r.set_shadow_casters(Some(&[caster]));
    r.set_render_experiments(options);
    for (w, h) in [(240, 160), (257, 171), (240, 160)] {
        for _ in 0..3 {
            r.capture(w, h, &frame, &[ground]).unwrap();
        }
        let resolve = r.cascade_resolve.as_ref().unwrap();
        assert_eq!(resolve.size, (w, h));
        let covered = r.read_raw_texture(resolve.mask.texture(), (w, h), 2).unwrap();
        assert!(covered.as_chunks::<2>().0.iter().any(|p| p[0] < 128 && p[1] == 255), "actual shadow coverage");
        r.capture(w, h, &FrameParams { camera_cut: true, ..frame }, &[ground]).unwrap();
        let resolve = r.cascade_resolve.as_ref().unwrap();
        let raw = r.read_raw_texture(resolve.raw.texture(), (w, h), 8).unwrap();
        let current = r.read_raw_texture(resolve.mask.texture(), (w, h), 2).unwrap();
        for (a, b) in raw.as_chunks::<8>().0.iter().zip(current.as_chunks::<2>().0) {
            let v = (half::f16::from_le_bytes([a[0], a[1]]).to_f32() * 255.0).round() as u8;
            assert!(v.abs_diff(b[0]) <= 1, "camera cut must use current visibility");
        }
    }
    r.set_shadow_casters(Some(&[]));
    r.capture(240, 160, &frame, &[ground]).unwrap();
    let resolve = r.cascade_resolve.as_ref().unwrap();
    let mask = r.read_raw_texture(resolve.mask.texture(), (240, 160), 2).unwrap();
    assert!(mask.as_chunks::<2>().0.iter().all(|p| p[0] == 255), "removed caster clears immediately");
    for active in [false, true, false, true] {
        r.set_render_experiments(RenderExperiments { cascade_temporal: active, ..options });
        r.capture(240, 160, &frame, &[ground]).unwrap();
    }
}
