//! Real installed contact textures delivered through the native shared effect layer.
use super::*;
use blackbox_gfx::{EffectLayer, FrameParams, Projection};
use blackbox_render::{Renderer, RendererOptions};
use std::rc::Rc;

#[test]
#[ignore = "requires an installed game and Vulkan GPU"]
fn installed_source_contact_particles_render_on_native_gpu() {
    let (dir, db) = tests::installed();
    let mut renderer = Renderer::headless(
        (640, 360),
        RendererOptions {
            backend: blackbox_gfx::GraphicsApi::Vulkan,
            vsync: false,
            force_fallback_adapter: false,
            pipeline_cache_dir: None,
        },
    )
    .unwrap();
    let mut graphics = *renderer.graphics();
    graphics.post = Default::default();
    graphics.ao.quality = blackbox_gfx::AoQuality::Off;
    graphics.shadows = blackbox_gfx::ShadowQuality::Off;
    graphics.reflections = blackbox_gfx::ReflectionQuality::Off;
    renderer.apply_graphics(&graphics);
    let eye = Vec3::new(4.0, -6.0, 3.0);
    let target = Vec3::Z * 0.5;
    let forward = (target - eye).normalize();
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, target, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse {
            fov_y: 45.0_f32.to_radians(),
            aspect: 16.0 / 9.0,
            near: 0.1,
        },
        camera_position: eye,
        light_dir: -Vec3::Z,
        clear_color: [0.12, 0.15, 0.18],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: true,
        shadow_tuning: None,
    };
    renderer.set_effects(&EffectLayer::default());
    renderer.render(&frame, &[]).unwrap();
    let before = renderer.read_output().unwrap().unwrap();
    let owner = Rc::new(());
    for (class, name, action, effect) in [
        ("smackable", "largemetalpost", Action::HitWorld, "lamp_lrg_hit_world"),
        ("smackable", "largemetalpost", Action::HitObject, "lamp_lrg_hit_car"),
        ("pvehicle", "copmidsize", Action::HitObject, "carhitcar"),
        ("pvehicle", "traftaxi", Action::HitWorld, "carhitwall"),
    ] {
        let mut fx = ImpactEffects::default();
        fx.step(&db, [tests::sample(&db, class, name, action, effect, &owner)], eye, 1.0 / 60.0);
        fx.maintain(&mut renderer, &dir);
        for _ in 0..3 {
            fx.step(&db, [], eye, 1.0 / 60.0);
        }
        let mut layer = EffectLayer::default();
        fx.textured(forward, &mut layer.textured);
        assert!(!layer.textured.is_empty(), "{effect} native texture delivery");
        renderer.set_effects(&layer);
        renderer.render(&frame, &[]).unwrap();
        let after = renderer.read_output().unwrap().unwrap();
        let changed = before
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .zip(after.rgba.as_chunks::<4>().0)
            .filter(|(a, b)| a[..3].iter().zip(&b[..3]).any(|(a, b)| a.abs_diff(*b) > 3))
            .count();
        assert!(changed > 10, "{effect}: native particles must produce visible pixels, found {changed}");
        eprintln!("{effect}: {changed} changed pixels");
        if let Some(path) = std::env::var_os("NFSMW_TEST_OUTPUT_DIR") {
            let path = std::path::PathBuf::from(path);
            std::fs::create_dir_all(&path).unwrap();
            let file =
                std::io::BufWriter::new(std::fs::File::create(path.join(format!("contact-{effect}.png"))).unwrap());
            let mut png = png::Encoder::new(file, after.width, after.height);
            png.set_color(png::ColorType::Rgba);
            png.set_depth(png::BitDepth::Eight);
            png.write_header().unwrap().write_image_data(&after.rgba).unwrap();
        }
        fx.release(&mut renderer);
    }
}
