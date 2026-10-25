use crate::{Renderer, gpu::test_support};
use blackbox_gfx::*;
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

#[test]
#[ignore = "requires Vulkan RT and BLACKBOX_RT_REFLECTIONS=1"]
fn hidden_casters_toggle_and_move_without_history_or_reflections() {
    if std::env::var("BLACKBOX_RT_REFLECTIONS").as_deref() != Ok("1")
        || test_support::test_api() != crate::Backend::Vulkan
    {
        return;
    }
    let _gpu = test_support::serial();
    let mut r = Renderer::headless((240, 160), test_support::options()).unwrap();
    assert!(r.ray_reflections.is_some());
    let mut m = MeshBuilder::new();
    m.state(None, BlendMode::Opaque, Shading::Prelit);
    m.flat_quad(
        [
            Vec3::new(-15.0, -15.0, 0.0),
            Vec3::new(15.0, -15.0, 0.0),
            Vec3::new(15.0, 15.0, 0.0),
            Vec3::new(-15.0, 15.0, 0.0),
        ],
        prelit([0.7; 3], 1.0),
    );
    let ground = Instance::new(m.finish().upload(&mut r, "sun ray ground"), Mat4::IDENTITY);
    let mut m = MeshBuilder::new();
    m.state(None, BlendMode::Opaque, Shading::Prelit);
    m.cuboid(Vec3::new(-2.0, -2.0, 4.0), Vec3::new(2.0, 2.0, 5.0), prelit([0.5; 3], 1.0));
    let blocker = Instance::new(m.finish().upload(&mut r, "hidden overhead caster"), Mat4::IDENTITY);
    let eye = Vec3::new(0.0, -7.0, 4.0);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 0.9, aspect: 1.5, near: 0.1 },
        camera_position: eye,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let options = RenderExperiments {
        sun_shadows: true,
        ray_sun_shadows: true,
        lighting_debug: LightingDebug::SunVisibility,
        ray_road_reflections: false,
        ray_car_reflections: false,
        ..Default::default()
    };
    r.set_render_experiments(options);
    r.set_shadow_casters(Some(&[]));
    let lit = r.capture(240, 160, &frame, &[ground]).unwrap();
    r.set_shadow_casters(Some(&[blocker]));
    let dark = r.capture(240, 160, &frame, &[ground]).unwrap();
    let mask = r.read_raw_texture(r.ray_shadows.as_ref().unwrap().mask.texture(), (240, 160), 2).unwrap();
    assert!(
        mask.as_chunks::<2>().0.iter().filter(|p| p[0] == 0 && p[1] == 255).count() > 300,
        "actual RT mask must contain full-weight occlusion"
    );
    let count = lit
        .as_chunks::<4>()
        .0
        .iter()
        .zip(dark.as_chunks::<4>().0)
        .filter(|(a, b)| a[0] > b[0].saturating_add(50))
        .count();
    assert!(count > 300, "off-screen caster must shadow visible receivers: {count}");
    assert_eq!(dark, r.capture(240, 160, &frame, &[ground]).unwrap(), "no frame noise/history");
    r.set_shadow_casters(Some(&[blocker.with_shadow(ShadowFlags { cast: false, receive: true })]));
    assert_eq!(lit, r.capture(240, 160, &frame, &[ground]).unwrap(), "non-casters must be excluded from rays");
    r.set_shadow_casters(Some(&[Instance::new(blocker.mesh, Mat4::from_translation(Vec3::X * 50.0))]));
    assert_eq!(lit, r.capture(240, 160, &frame, &[ground]).unwrap(), "moving a caster clears old shadows");
    r.set_render_experiments(RenderExperiments { ray_sun_shadows: false, ..options });
    assert_eq!(lit, r.capture(240, 160, &frame, &[ground]).unwrap(), "off restores cascades without stale mask");
}

#[test]
#[ignore = "requires Vulkan RT and BLACKBOX_RT_REFLECTIONS=1"]
fn cutout_families_and_range_fade_match_the_shadow_contract() {
    if std::env::var("BLACKBOX_RT_REFLECTIONS").as_deref() != Ok("1")
        || test_support::test_api() != crate::Backend::Vulkan
    {
        return;
    }
    let _gpu = test_support::serial();
    let mut r = Renderer::headless((240, 160), test_support::options()).unwrap();
    let mut m = MeshBuilder::new();
    m.state(None, BlendMode::Opaque, Shading::Prelit);
    m.flat_quad(
        [
            Vec3::new(-15.0, -15.0, 0.0),
            Vec3::new(15.0, -15.0, 0.0),
            Vec3::new(15.0, 15.0, 0.0),
            Vec3::new(-15.0, 15.0, 0.0),
        ],
        prelit([0.7; 3], 1.0),
    );
    let ground = Instance::new(m.finish().upload(&mut r, "sun ray ground"), Mat4::IDENTITY);
    let mut m = MeshBuilder::new();
    m.state(None, BlendMode::Opaque, Shading::Prelit);
    m.cuboid(Vec3::new(-2.0, -2.0, 4.0), Vec3::new(2.0, 2.0, 5.0), prelit([0.5; 3], 1.0));
    let blocker = Instance::new(m.finish().upload(&mut r, "hidden overhead caster"), Mat4::IDENTITY);
    let eye = Vec3::new(0.0, -7.0, 4.0);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 0.9, aspect: 1.5, near: 0.1 },
        camera_position: eye,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };

    r.destroy_mesh(blocker.mesh);
    let options = RenderExperiments {
        sun_shadows: true,
        ray_sun_shadows: true,
        original_world_materials: true,
        lighting_debug: LightingDebug::SunVisibility,
        ..Default::default()
    };
    for (shading, vertex_alpha, texture_alpha, original, blocked) in [
        (Shading::World, 127.0 / 255.0, 255, true, true),
        (Shading::World, 127.0 / 255.0, 255, false, true),
        (Shading::World, 1.0, 0, true, false),
        (Shading::Prelit, 0.3, 255, true, false),
        (Shading::Tree, 0.3, 255, true, true),
        (Shading::WorldPrelit, 0.3, 255, true, true),
        (Shading::Tree, 0.3, 255, false, false),
        (Shading::WorldPrelit, 1.0, 0, true, false),
    ] {
        let bytes = [255, 255, 255, texture_alpha];
        let texture = r.create_texture(&TextureDesc {
            label: "sun alpha",
            width: 1,
            height: 1,
            format: PixelFormat::Rgba8,
            mips: vec![&bytes],
        });
        let mut m = MeshBuilder::new();
        m.state(Some(texture), BlendMode::AlphaTest, shading);
        m.flat_quad(
            [
                Vec3::new(-2.0, -2.0, 4.0),
                Vec3::new(2.0, -2.0, 4.0),
                Vec3::new(2.0, 2.0, 4.0),
                Vec3::new(-2.0, 2.0, 4.0),
            ],
            prelit([0.7; 3], vertex_alpha),
        );
        let caster = Instance::new(m.finish().upload(&mut r, "alpha shadow roof"), Mat4::IDENTITY);
        r.set_render_experiments(RenderExperiments { original_world_materials: original, ..options });
        r.set_shadow_casters(Some(&[caster]));
        r.capture(240, 160, &frame, &[ground]).unwrap();
        let mask = r.read_raw_texture(r.ray_shadows.as_ref().unwrap().mask.texture(), (240, 160), 2).unwrap();
        let count = mask.as_chunks::<2>().0.iter().filter(|p| p[0] == 0 && p[1] == 255).count();
        assert_eq!(count > 300, blocked, "alpha family {shading:?}, original={original}: {count}");
        r.destroy_mesh(caster.mesh);
        r.destroy_texture(texture);
    }
    r.set_shadow_casters(Some(&[]));
    r.set_render_experiments(RenderExperiments { ray_shadow_range: 10.0, ..options });
    r.capture(240, 160, &frame, &[ground]).unwrap();
    let mask = r.read_raw_texture(r.ray_shadows.as_ref().unwrap().mask.texture(), (240, 160), 2).unwrap();
    assert!(mask.as_chunks::<2>().0.iter().any(|p| p[1] > 0 && p[1] < 255), "range handoff has intermediate weights");
    assert!(mask.as_chunks::<2>().0.iter().any(|p| p[1] == 0), "far pixels retain cascades");
}
