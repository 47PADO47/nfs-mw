//! Native renderer evidence for installed police replacement meshes and health particles.
use super::*;
use crate::scenes::world::damage_effects::{DamageEffects, Source};
use blackbox_gfx::{EffectLayer, FrameParams, Projection, RenderBackend, RgbaImage};
use blackbox_render::{Renderer, RendererOptions};
use nfsmw_data::car::damage_parts;
use std::{collections::BTreeSet, rc::Rc};

fn save(image: &RgbaImage, name: &str) {
    let Some(path) = std::env::var_os("NFSMW_TEST_OUTPUT_DIR") else { return };
    let path = std::path::PathBuf::from(path);
    std::fs::create_dir_all(&path).unwrap();
    let file = std::io::BufWriter::new(std::fs::File::create(path.join(format!("cop-{name}.png"))).unwrap());
    let mut png = png::Encoder::new(file, image.width, image.height);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header().unwrap().write_image_data(&image.rgba).unwrap();
}

#[test]
#[ignore = "requires an installed game and Vulkan GPU"]
fn installed_heat_one_cop_damage_and_health_particles_render_on_native_gpu() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let model = nfsmw_data::car::load_driving(
        &dir,
        "COPMIDSIZE",
        &nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None },
    )
    .unwrap();
    let mut renderer = Renderer::headless(
        (960, 540),
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
    let rig = CarRig::upload(
        &mut renderer,
        keys::next_car(),
        model,
        CarShading::Original,
        Vec3::new(1.0, 1.0, 2.0).normalize(),
    );
    let pose = CarPose { position: Vec3::ZERO, rotation: Quat::IDENTITY, wheels: [Default::default(); 4] };
    let eye = Vec3::new(6.5, 6.0, 3.5);
    let target = Vec3::Z * 0.8;
    let forward = (target - eye).normalize();
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, target, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse {
            fov_y: 45.0_f32.to_radians(),
            aspect: 16.0 / 9.0,
            near: 0.1,
        },
        camera_position: eye,
        light_dir: -Vec3::new(1.0, 1.0, 2.0).normalize(),
        clear_color: [0.12, 0.15, 0.18],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: true,
        shadow_tuning: None,
    };
    let mut fx = DamageEffects::default();
    let owner = Rc::new(());
    let mut levels = [0; 10];
    let mut captures = Vec::new();
    for (name, health) in [("pristine", 1.0), ("damaged", 1.0), ("smoke", 0.5), ("destroyed", 0.0)] {
        if name != "pristine" {
            levels[0] = 2;
            levels[2] = 2;
        }
        let source = || Source {
            id: 7,
            owner: Rc::downgrade(&owner),
            fx: &rig.model.driving_lods.damage_fx,
            health,
            frame: blackbox_particles::Frame::default(),
        };
        fx.step([source()], 1.0 / 60.0);
        fx.maintain(&mut renderer, &dir);
        for _ in 0..120 {
            fx.step([source()], 1.0 / 60.0);
        }
        let glass = damage_parts::glass_mask(&levels);
        rig.prepare_glass(&mut renderer, &BTreeSet::from([glass]), Vec3::new(1.0, 1.0, 2.0).normalize());
        let mut instances = Vec::new();
        rig.instances_detail(
            &pose,
            None,
            &BTreeSet::new(),
            super::super::DrawOptions { car: 7, glass, damage: Some(&levels), ..Default::default() },
            &mut instances,
        );
        let mut layer = EffectLayer::default();
        fx.textured(eye, forward, &mut layer.textured);
        if health <= 0.5 {
            assert!(!layer.textured.is_empty(), "installed health particles must enter native draw batches");
        }
        renderer.set_effects(&layer);
        renderer.render(&frame, &instances).unwrap();
        let image = renderer.read_output().unwrap().unwrap();
        save(&image, name);
        captures.push(image);
    }
    for (pair, label) in captures.windows(2).zip(["authored model damage", "damage smoke", "death group"]) {
        let changed = pair[0]
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .zip(pair[1].rgba.as_chunks::<4>().0)
            .filter(|(a, b)| a[..3].iter().zip(&b[..3]).any(|(a, b)| a.abs_diff(*b) > 3))
            .count();
        assert!(changed > 100, "{label} needs visible pixel changes, found {changed}");
        eprintln!("COPMIDSIZE {label}: {changed} changed pixels");
    }
    fx.release(&mut renderer);
    rig.release(&mut renderer);
}
