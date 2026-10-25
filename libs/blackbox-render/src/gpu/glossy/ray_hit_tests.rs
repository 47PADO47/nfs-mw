//! Secondary car hits use the source material, not just its raw paint texture.
use crate::{Renderer, gpu::test_support};
use blackbox_gfx::*;
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};
#[test]
#[ignore = "requires Vulkan RT and BLACKBOX_RT_REFLECTIONS=1"]
fn secondary_car_hit_respects_material_colour_metal_and_live_slot_reuse() {
    if std::env::var("BLACKBOX_RT_REFLECTIONS").as_deref() != Ok("1")
        || test_support::test_api() != crate::Backend::Vulkan
    {
        return;
    }
    let _serial = test_support::serial();
    let mut r = Renderer::headless((257, 193), test_support::options()).unwrap();
    let black = [0, 0, 0, 255];
    r.set_environment(Environment::Faces { size: 1, faces: [&black; 6] });
    let mirror_material = r.create_glossy_material(&GlossyMaterial {
        diffuse_min: [0.0, 0.0, 0.0, 1.0],
        diffuse_range: [0.0; 4],
        specular_min: [0.0; 3],
        specular_range: [0.0; 3],
        envmap_min: 2.0,
        envmap_range: 0.0,
        ..Default::default()
    });
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Glossy(mirror_material));
    mesh.flat_quad(
        [
            Vec3::new(-30.0, -30.0, 0.0),
            Vec3::new(30.0, -30.0, 0.0),
            Vec3::new(30.0, 30.0, 0.0),
            Vec3::new(-30.0, 30.0, 0.0),
        ],
        prelit([1.0; 3], 1.0),
    );
    let mirror = Instance::new(mesh.finish().upload(&mut r, "secondary car hit mirror"), Mat4::IDENTITY);
    let actor_material = r.create_glossy_material(&GlossyMaterial {
        diffuse_min: [0.0, 0.6, 0.0, 1.0],
        diffuse_range: [0.0; 4],
        surface: Some(GlossySurface { kind: SurfaceKind::Diffuse, roughness: 0.7, coat: 0.0 }),
        ..Default::default()
    });
    let yellow = [230, 230, 0, 255];
    let texture = r.create_texture(&TextureDesc {
        label: "yellow source skin",
        width: 1,
        height: 1,
        format: PixelFormat::Rgba8,
        mips: vec![&yellow],
    });
    let mut mesh = MeshBuilder::new();
    mesh.state(Some(texture), BlendMode::Opaque, Shading::Glossy(actor_material));
    mesh.flat_quad(
        [
            Vec3::new(-8.0, 16.0, 6.0),
            Vec3::new(-8.0, 16.0, 10.0),
            Vec3::new(8.0, 16.0, 10.0),
            Vec3::new(8.0, 16.0, 6.0),
        ],
        prelit([1.0; 3], 1.0),
    );
    let actor = Instance::new(mesh.finish().upload(&mut r, "material hit actor"), Mat4::IDENTITY);
    let eye = Vec3::new(0.0, -8.0, 6.0);
    let f = FrameParams {
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
    r.set_render_experiments(RenderExperiments {
        ray_car_reflections: true,
        paint_response: true,
        ..Default::default()
    });
    r.render(&f, &[mirror]).unwrap();
    let absent = r.read_output().unwrap().unwrap();
    r.set_ray_reflection_instances(&[actor]);
    r.render(&f, &[mirror]).unwrap();
    let green = r.read_output().unwrap().unwrap();
    assert!(
        green
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[1] > p[0].saturating_add(15) && p[1] > p[2].saturating_add(15))
            .count()
            > 100,
        "green material must modulate the yellow source skin"
    );
    r.destroy_glossy_material(actor_material);
    r.render(&f, &[mirror]).unwrap();
    assert_eq!(absent.rgba, r.read_output().unwrap().unwrap().rgba, "destroyed hit materials are skipped");
    let chrome = r.create_glossy_material(&GlossyMaterial {
        diffuse_min: [0.0, 0.0, 0.0, 1.0],
        surface: Some(GlossySurface { kind: SurfaceKind::Conductor, roughness: 0.12, coat: 0.0 }),
        ..Default::default()
    });
    assert_eq!(chrome, actor_material, "exercise live material slot reuse without rebuilding actor geometry");
    r.render(&f, &[mirror]).unwrap();
    assert_eq!(
        absent.rgba,
        r.read_output().unwrap().unwrap().rgba,
        "metal in a black environment must not emit raw yellow paint"
    );
}
