//! Hardware car rays must work without wet roads, SSR or a visible reflected object.
use crate::{Renderer, gpu::test_support};
use blackbox_gfx::*;
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

#[test]
#[ignore = "requires Vulkan RT and BLACKBOX_RT_REFLECTIONS=1"]
fn car_rt_reflects_hidden_geometry_on_opaque_and_glass_and_keeps_rough_and_original() {
    if std::env::var("BLACKBOX_RT_REFLECTIONS").as_deref() != Ok("1")
        || test_support::test_api() != crate::Backend::Vulkan
    {
        return;
    }
    let _gpu = test_support::serial();
    let mut r = Renderer::headless((257, 193), test_support::options()).unwrap();
    assert!(r.ray_reflections.is_some());
    let cube = [0, 0, 0, 255];
    r.set_environment(Environment::Faces { size: 1, faces: [&cube; 6] });
    let mut wall = MeshBuilder::new();
    wall.state(None, BlendMode::Opaque, Shading::Prelit);
    wall.flat_quad(
        [
            Vec3::new(-8.0, 16.0, 6.0),
            Vec3::new(-8.0, 16.0, 10.0),
            Vec3::new(8.0, 16.0, 10.0),
            Vec3::new(8.0, 16.0, 6.0),
        ],
        prelit([0.9, 0.0, 0.0], 1.0),
    );
    let wall = Instance::new(wall.finish().upload(&mut r, "hidden car reflection wall"), Mat4::IDENTITY);
    let eye = Vec3::new(0.0, -8.0, 6.0);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 0.8, aspect: 257.0 / 193.0, near: 0.1 },
        camera_position: eye,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    for (kind, roughness, mode, model, should_reflect) in [
        (SurfaceKind::Conductor, 0.08, BlendMode::Opaque, GlossyModel::Custom, true),
        (SurfaceKind::Glass, 0.08, BlendMode::AlphaBlend, GlossyModel::Custom, true),
        (SurfaceKind::Legacy, 0.8, BlendMode::Opaque, GlossyModel::Custom, false),
        (SurfaceKind::Conductor, 0.08, BlendMode::Opaque, GlossyModel::VertexLit, false),
    ] {
        let material = r.create_glossy_material(&GlossyMaterial {
            model,
            surface: Some(GlossySurface { kind, roughness, coat: 0.0 }),
            diffuse_min: [0.0, 0.0, 0.0, 0.5],
            diffuse_range: [0.0; 4],
            specular_min: [0.0; 3],
            specular_range: [0.0; 3],
            envmap_min: 2.0,
            envmap_range: 0.0,
            ..Default::default()
        });
        let mut mirror = MeshBuilder::new();
        mirror.state(None, mode, Shading::Glossy(material));
        mirror.flat_quad(
            [
                Vec3::new(-30.0, -30.0, 0.0),
                Vec3::new(30.0, -30.0, 0.0),
                Vec3::new(30.0, 30.0, 0.0),
                Vec3::new(-30.0, 30.0, 0.0),
            ],
            prelit([1.0; 3], 1.0),
        );
        let mirror = Instance::new(mirror.finish().upload(&mut r, "car RT receiver"), Mat4::IDENTITY);
        r.set_render_experiments(RenderExperiments {
            ray_car_reflections: false,
            paint_response: true,
            ..Default::default()
        });
        r.render(&frame, &[mirror]).unwrap();
        let absent = r.read_output().unwrap().unwrap();
        r.set_ray_reflection_instances(&[wall]);
        let options = RenderExperiments { ray_car_reflections: true, paint_response: true, ..Default::default() };
        r.set_render_experiments(options);
        r.render(&frame, &[mirror]).unwrap();
        let hit = r.read_output().unwrap().unwrap();
        if !should_reflect {
            assert_eq!(absent.rgba, hit.rgba, "rough/original materials must retain their response: {kind:?}");
            r.destroy_mesh(mirror.mesh);
            continue;
        }
        let count = hit
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .zip(absent.rgba.as_chunks::<4>().0.iter())
            .filter(|(b, a)| b[0] > a[0].saturating_add(5) && b[0] > b[1].saturating_add(5))
            .count();
        assert!(count > 100, "RT must reach hidden geometry on {kind:?}: {count}");
        if kind == SurfaceKind::Conductor {
            for ray_texture_quality in [0, 3, 1] {
                r.set_render_experiments(RenderExperiments { ray_texture_quality, ..options });
                r.render(&frame, &[mirror]).unwrap();
                assert_eq!(
                    hit.rgba,
                    r.read_output().unwrap().unwrap().rgba,
                    "cache replacement must preserve hit bindings"
                );
            }
        }
        r.set_render_experiments(RenderExperiments { ray_car_blend: 0.0, ..options });
        r.render(&frame, &[mirror]).unwrap();
        assert_eq!(absent.rgba, r.read_output().unwrap().unwrap().rgba, "zero contribution restores the probe");
        r.set_render_experiments(options);
        r.set_ray_reflection_instances(&[]);
        r.render(&frame, &[mirror]).unwrap();
        assert_eq!(absent.rgba, r.read_output().unwrap().unwrap().rgba, "removing hidden geometry leaves no history");
        r.destroy_mesh(mirror.mesh);
    }
}
