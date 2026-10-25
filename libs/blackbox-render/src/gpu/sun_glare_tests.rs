//! Main-view solar glare must follow the source and disappear behind geometry.
use crate::gpu::test_support;
use blackbox_gfx::*;
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};
#[test]
#[ignore = "needs a native GPU"]
fn source_occlusion_weather_and_disable_control_solar_glare() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((320, 200)) else { return };
    let texture = r.create_texture(&TextureDesc {
        label: "synthetic glare artwork",
        width: 1,
        height: 1,
        format: PixelFormat::Rgba8,
        mips: vec![&[255; 4]],
    });
    let sun = Vec3::new(0.0, 1.0, 0.25).normalize();
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::Y, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.6, near: 0.1 },
        camera_position: Vec3::ZERO,
        light_dir: -sun,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let state = SunGlare {
        layers: vec![SunGlareLayer { texture, radius: 0.15, brightness: 1.0, rotation: 0.0 }],
        colour: [1.0; 3],
        strength: 1.0,
        weather_visibility: 1.0,
        angular_radius: 0.5,
    };
    r.set_sun_glare(Some(state.clone()));
    let visible = r.capture(320, 200, &frame, &[]).unwrap();
    let lit = |image: &[u8]| image.as_chunks::<4>().0.iter().filter(|p| p[0] > 20).count();
    assert!(lit(&visible) > 1000, "the sun artwork is visible");
    r.set_sun_glare(Some(SunGlare { weather_visibility: 0.0, ..state.clone() }));
    let clouded = r.capture(320, 200, &frame, &[]).unwrap();
    assert_eq!(lit(&clouded), 0, "overcast suppresses fallback glare");
    r.set_sun_glare(Some(state));
    let mut m = MeshBuilder::new();
    m.state(None, BlendMode::Opaque, Shading::Prelit);
    m.flat_quad(
        [Vec3::new(-0.4, 2.0, 0.1), Vec3::new(0.4, 2.0, 0.1), Vec3::new(0.4, 2.0, 0.9), Vec3::new(-0.4, 2.0, 0.9)],
        prelit([0.0; 3], 1.0),
    );
    let wall = Instance::new(m.finish().upload(&mut r, "small solar occluder"), Mat4::IDENTITY);
    let hidden = r.capture(320, 200, &frame, &[wall]).unwrap();
    assert_eq!(lit(&hidden), 0, "a small central blocker hides the whole optical glare, not just its middle");
    let restored = r.capture(320, 200, &frame, &[]).unwrap();
    assert_eq!(visible, restored, "no delayed glare history");
    r.set_sun_glare(None);
    assert_eq!(lit(&r.capture(320, 200, &frame, &[]).unwrap()), 0, "disable clears the effect");
}
