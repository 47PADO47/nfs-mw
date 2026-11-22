use super::*;
use crate::scenes::world::drive::{DetailView, DrawOptions};
use crate::{
    app::test_backend::NullBackend,
    settings::{CarGeometry, DetailSettings},
};
use blackbox_scenery::LodView;

#[test]
#[ignore = "requires an installed game"]
fn installed_car_detail_switches_cached_meshes_without_uploads_or_actor_mutation() {
    let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut changed = 0;
    for name in ["ELISE", "BMWM3GTR", "COPMIDSIZE", "COPGTO", "SEMI", "TRAILERB"] {
        let model = nfsmw_data::car::load_driving(
            &dir,
            name,
            &nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None },
        )
        .unwrap();
        assert!(!model.placements.is_empty());
        let floor = model.floor_height;
        let canonical = model.placements.clone();
        let mut backend = NullBackend::new(crate::settings::test_caps::native())
            .with_mesh_tracking()
            .with_texture_tracking()
            .with_glossy_tracking();
        let rig = CarRig::upload(&mut backend, keys::next_car(), model, CarShading::Original, Vec3::Z);
        let before = {
            let r = backend.resources.lock().unwrap();
            (r.meshes, r.textures)
        };
        let pose = CarPose { position: Vec3::ZERO, rotation: Quat::IDENTITY, wheels: [WheelPose::default(); 4] };
        let near = DetailView {
            view: LodView { position: [0.0, -5.0, 0.0], forward: [0.0, 1.0, 0.0], pixel_scale: 343.0 },
            settings: DetailSettings { car: CarGeometry::High, ..Default::default() },
        };
        let far = DetailView { view: LodView { position: [0.0, -100.0, 0.0], ..near.view }, ..near };
        let mut full = Vec::new();
        rig.instances(&pose, None, &Default::default(), &mut full);
        let mut distant = Vec::new();
        rig.instances_detail(
            &pose,
            None,
            &Default::default(),
            DrawOptions { car: rig.car, detail: Some(&far), ..Default::default() },
            &mut distant,
        );
        assert!(!distant.is_empty(), "{name} retains usable body geometry");
        changed += usize::from(
            distant.iter().map(|i| i.mesh).collect::<Vec<_>>() != full.iter().map(|i| i.mesh).collect::<Vec<_>>(),
        );
        for view in [near, far, near, far] {
            let mut out = Vec::new();
            rig.instances_detail(
                &pose,
                None,
                &Default::default(),
                DrawOptions { car: rig.car, detail: Some(&view), ..Default::default() },
                &mut out,
            );
            let mut keys: Vec<_> = out.iter().map(|i| i.key).collect();
            keys.sort();
            keys.dedup();
            assert_eq!(keys.len(), out.len(), "{name} needs one stable identity per part");
            let hidden: std::collections::BTreeSet<_> = rig.model.placements.iter().map(|p| p.slot).collect();
            let mut removed = Vec::new();
            rig.instances_detail(
                &pose,
                Some(&hidden),
                &Default::default(),
                DrawOptions { car: rig.car, detail: Some(&view), ..Default::default() },
                &mut removed,
            );
            assert!(removed.is_empty(), "hidden canonical slots stay hidden at every level");
        }
        let mut independent = Vec::new();
        rig.instances(&pose, None, &Default::default(), &mut independent);
        let geometry = |list: &[Instance]| list.iter().map(|i| (i.mesh, i.key, i.transform)).collect::<Vec<_>>();
        assert_eq!(
            geometry(&full),
            geometry(&independent),
            "main-view detail must not leak into shadow/reflection submissions"
        );
        assert_eq!(rig.model.floor_height, floor);
        assert_eq!(rig.model.placements, canonical);
        assert_eq!(
            {
                let r = backend.resources.lock().unwrap();
                (r.meshes, r.textures)
            },
            before
        );
        rig.release(&mut backend);
        let counts = backend.resources.lock().unwrap();
        assert_eq!((counts.meshes, counts.textures), (0, 0));
    }
    assert!(changed >= 3, "fixtures must exercise actual lower meshes");
}

#[test]
#[ignore = "requires the installed container trailer and a Vulkan GPU"]
fn installed_container_panels_occlude_the_background() {
    use blackbox_gfx::{FrameParams, GraphicsApi, Projection};
    use blackbox_render::{Renderer, RendererOptions};
    let mut dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    if let Some(path) = std::env::var_os("NFSMW_TEST_TEXTURE_OVERLAY") {
        dir.add_overlay("texture-test", path, 0).unwrap();
    }
    let model = nfsmw_data::car::load_driving(
        &dir,
        "TRAILERCON",
        &nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None },
    )
    .unwrap();
    let size = (3840, 2160);
    let mut renderer = Renderer::headless(
        size,
        RendererOptions {
            backend: GraphicsApi::Vulkan,
            vsync: false,
            force_fallback_adapter: false,
            pipeline_cache_dir: None,
        },
    )
    .unwrap();
    let rig = CarRig::upload_traffic(&mut renderer, 1, model, CarShading::Glossy, Vec3::Z);
    let pose = CarPose { position: Vec3::ZERO, rotation: Quat::IDENTITY, wheels: [Default::default(); 4] };
    let mut instances = Vec::new();
    rig.instances(&pose, None, &Default::default(), &mut instances);
    let eye = Vec3::new(-8.0, 12.0, 4.0);
    let mut frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::Z * 1.8, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 0.8, aspect: 16.0 / 9.0, near: 0.1 },
        camera_position: eye,
        light_dir: -Vec3::Z,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: true,
        shadow_tuning: None,
    };
    let mut images = Vec::new();
    for (label, bg) in [("black", [0.0; 3]), ("white", [1.0; 3])] {
        frame.clear_color = bg;
        renderer.render(&frame, &instances).unwrap();
        let image = renderer.read_output().unwrap().unwrap();
        if let Some(path) = std::env::var_os("NFSMW_TEST_OUTPUT_DIR") {
            let path = std::path::Path::new(&path).join(format!("container-{label}.png"));
            let mut png =
                png::Encoder::new(std::io::BufWriter::new(std::fs::File::create(path).unwrap()), size.0, size.1);
            png.set_color(png::ColorType::Rgba);
            png.set_depth(png::BitDepth::Eight);
            png.write_header().unwrap().write_image_data(&image.rgba).unwrap();
        }
        images.push(image);
    }
    assert_ne!(images[0].pixel(0, 0), images[1].pixel(0, 0));
    for y in size.1 / 2 - 64..size.1 / 2 + 64 {
        for x in size.0 / 2 - 128..size.0 / 2 + 128 {
            assert_eq!(images[0].pixel(x, y), images[1].pixel(x, y), "container panel at {x},{y}");
        }
    }
    rig.release(&mut renderer);
}
