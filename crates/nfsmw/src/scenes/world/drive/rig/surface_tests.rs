//! Installed texture admission and per-actor surface-cache material output.
use super::*;
use crate::app::test_backend::NullBackend;
use crate::scenes::world::drive::DrawOptions;
use blackbox_hash::bstring_hash as hash;
use std::collections::BTreeSet;

fn model() -> CarModel {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    nfsmw_data::car::load_driving(
        &dir,
        "BMWM3GTR",
        &nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None },
    )
    .unwrap()
}

#[test]
#[ignore = "requires installed BMW textures (set NFSMW_GAME_DIR)"]
fn installed_broken_brake_swaps_preserve_individual_sides_and_head_textures() {
    let model = model();
    let on = hash("BMWM3GTR_KIT00_BRAKELIGHT_ON");
    let off = hash("BMWM3GTR_KIT00_BRAKELIGHT_OFF");
    assert_ne!(on, off);
    assert!(model.textures.contains_key(&on) && model.textures.contains_key(&off));
    let baseline = model.swaps.clone().with_lamp_state(true, true, &model.textures);
    for broken in [8, 16, 32, 8 | 32, 56, 1 | 2 | 4] {
        let swaps = model.swaps.clone().with_broken_brake_lights(broken).with_lamp_state(true, true, &model.textures);
        for (side, bit) in [("LEFT", 8), ("RIGHT", 16), ("CENTRE", 32)] {
            for kind in ["BRAKELIGHT", "BRAKELIGHT_GLASS"] {
                let placeholder = hash(&format!("{kind}_{side}"));
                let state = if broken & bit != 0 { "OFF" } else { "ON" };
                let wanted = hash(&format!("BMWM3GTR_KIT00_{kind}_{state}"));
                let expected = match model.textures.contains_key(&wanted) {
                    true => wanted,
                    false => model.swaps.resolve(placeholder),
                };
                assert_eq!(swaps.resolve(placeholder), expected, "{kind} {side} broken={broken}");
            }
        }
        for kind in ["HEADLIGHT", "HEADLIGHT_GLASS"] {
            for side in ["LEFT", "RIGHT"] {
                let placeholder = hash(&format!("{kind}_{side}"));
                assert_eq!(swaps.resolve(placeholder), baseline.resolve(placeholder));
            }
        }
    }
    let mut missing = model.textures.clone();
    missing.remove(&off);
    let mut seeded = model.swaps.clone();
    seeded.map.insert(hash("BRAKELIGHT_LEFT"), on);
    let fallback = seeded.with_broken_brake_lights(8).with_lamp_state(true, true, &missing);
    assert_eq!(fallback.resolve(hash("BRAKELIGHT_LEFT")), on, "missing OFF art preserves the available replacement");
    assert_eq!(fallback.resolve(hash("BRAKELIGHT_RIGHT")), on);
}

fn draw(rig: &CarRig, actor: u32, glass: u16, broken: u32, lamps: [bool; 2]) -> Vec<Instance> {
    let pose = CarPose { position: Vec3::ZERO, rotation: Quat::IDENTITY, wheels: [Default::default(); 4] };
    let mut out = Vec::new();
    rig.instances_detail(
        &pose,
        None,
        &BTreeSet::new(),
        DrawOptions { car: actor, glass, broken_lights: broken, lamps, ..Default::default() },
        &mut out,
    );
    out
}

/// Inspect the production upload's actual texture handles, not only a swap map.
fn assert_materials(
    rig: &CarRig,
    backend: &NullBackend,
    actor: u32,
    drawn: &[Instance],
    swaps: &TextureSwaps,
) -> usize {
    let appearance = rig.appearance.borrow();
    let mut checked = 0;
    for part in appearance.parts.iter().filter(|part| part.lod == 0 && !part.damaged) {
        let Some(instance) = drawn.iter().find(|instance| instance.key == keys::car_part(actor, part.key)) else {
            continue;
        };
        let solid = &rig.model.solids[&part.placement.solid];
        let expected = solid
            .groups
            .iter()
            .filter(|group| group.diffuse_texture(solid).is_none_or(|h| !swaps.hidden.contains(&h)))
            .map(|group| {
                group.diffuse_texture(solid).and_then(|h| {
                    appearance
                        .materials
                        .texture(swaps.resolve_for(h, part.placement.left_brake))
                        .or_else(|| appearance.materials.texture(h))
                })
            })
            .collect::<Vec<_>>();
        let actual = backend.mesh_draws[&instance.mesh].iter().map(|group| group.texture).collect::<Vec<_>>();
        assert_eq!(actual, expected, "actor {actor} solid {}", solid.name);
        checked += actual.len();
    }
    checked
}

fn counts(backend: &NullBackend) -> (usize, usize, usize) {
    let resources = backend.resources.lock().unwrap();
    (resources.meshes, resources.textures, backend.glossy_materials.len())
}

#[test]
#[ignore = "requires installed Supra and NFSMW_EXPERIMENTAL_LAMPS=1"]
fn installed_lamp_glass_does_not_emit_through_shared_body_slot() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let model = nfsmw_data::car::load_driving(
        &dir,
        "SUPRA",
        &nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None },
    )
    .unwrap();
    let mut backend = NullBackend::new(crate::settings::test_caps::native())
        .with_mesh_tracking()
        .with_glossy_tracking()
        .with_normal_tracking();
    let rig = CarRig::upload(&mut backend, keys::next_car(), model, CarShading::Glossy, Vec3::Z);
    assert!(rig.appearance.borrow().materials.enhanced_lamps());
    let appearance = rig.appearance.borrow();
    let mut seen = false;
    for part in appearance.parts.iter().filter(|p| p.lod == 0 && !p.damaged) {
        let solid = &rig.model.solids[&part.placement.solid];
        if !solid.name.contains("HEADLIGHT_GLASS") {
            continue;
        }
        for (group, draw) in solid
            .groups
            .iter()
            .filter(|g| g.diffuse_texture(solid).is_none_or(|h| !rig.model.swaps.hidden.contains(&h)))
            .zip(&backend.mesh_draws[&part.lamps[1]])
        {
            if group.diffuse_texture(solid) != Some(hash("HEADLIGHT_LEFT")) {
                continue;
            }
            let Shading::Glossy(material) = draw.shading else { panic!("glossy glass expected") };
            assert_ne!(
                backend.glossy_materials[&material].surface.map(|s| s.kind),
                Some(blackbox_gfx::SurfaceKind::Emissive)
            );
            seen = true;
        }
    }
    assert!(seen, "a placed Supra glass mesh must use the headlight body slot");
    drop(appearance);
    rig.release(&mut backend);
}

#[test]
#[ignore = "requires installed BMW and NFSMW_EXPERIMENTAL_LAMPS=1"]
fn installed_experimental_head_damage_is_per_actor_and_repairs() {
    let mut backend = NullBackend::new(crate::settings::test_caps::native())
        .with_mesh_tracking()
        .with_texture_tracking()
        .with_glossy_tracking()
        .with_normal_tracking();
    let rig = CarRig::upload(&mut backend, keys::next_car(), model(), CarShading::Glossy, Vec3::Z);
    assert!(rig.appearance.borrow().materials.enhanced_lamps());
    let intact_swaps = rig.swaps().with_lamp_state(true, true, &rig.model.textures);
    let left_swaps = rig.swaps().with_broken_head_lights(1).with_lamp_state(true, true, &rig.model.textures);
    assert!(intact_swaps.lamp_is_on(hash("HEADLIGHT_LEFT")));
    assert!(!left_swaps.lamp_is_on(hash("HEADLIGHT_LEFT")));
    assert!(left_swaps.lamp_is_on(hash("HEADLIGHT_RIGHT")));
    rig.prepare_surfaces(&mut backend, &BTreeSet::from([SurfaceDamage::new(0, 1)]), Vec3::Z);
    let intact = draw(&rig, 7, 0, 0, [true, true]);
    let broken = draw(&rig, 8, 0, 1, [true, true]);
    assert!(assert_materials(&rig, &backend, 7, &intact, &intact_swaps) > 0);
    assert!(assert_materials(&rig, &backend, 8, &broken, &left_swaps) > 0);
    let appearance = rig.appearance.borrow();
    let part = appearance
        .parts
        .iter()
        .find(|part| {
            part.lod == 0
                && !part.damaged
                && rig.model.solids[&part.placement.solid].groups.iter().any(|g| {
                    g.diffuse_texture(&rig.model.solids[&part.placement.solid]) == Some(hash("HEADLIGHT_LEFT"))
                })
        })
        .expect("BMW must have a left headlight slot");
    let solid = &rig.model.solids[&part.placement.solid];
    let group_index = solid
        .groups
        .iter()
        .filter(|g| g.diffuse_texture(solid).is_none_or(|h| !rig.swaps().hidden.contains(&h)))
        .position(|g| g.diffuse_texture(solid) == Some(hash("HEADLIGHT_LEFT")))
        .unwrap();
    let surface = |drawn: &[Instance], actor| {
        let instance = drawn.iter().find(|i| i.key == keys::car_part(actor, part.key)).unwrap();
        let Shading::Glossy(material) = backend.mesh_draws[&instance.mesh][group_index].shading else {
            panic!("glossy lamp expected")
        };
        backend.glossy_materials[&material].surface.map(|s| s.kind)
    };
    assert_eq!(surface(&intact, 7), Some(blackbox_gfx::SurfaceKind::Emissive));
    assert_ne!(surface(&broken, 8), Some(blackbox_gfx::SurfaceKind::Emissive));
    drop(appearance);
    assert!(intact.iter().zip(&broken).any(|(a, b)| a.mesh != b.mesh));
    rig.prepare_surfaces(&mut backend, &BTreeSet::new(), Vec3::Z);
    let repaired = draw(&rig, 8, 0, 0, [true, true]);
    assert_eq!(intact.iter().map(|i| i.mesh).collect::<Vec<_>>(), repaired.iter().map(|i| i.mesh).collect::<Vec<_>>());
    rig.release(&mut backend);
    assert_eq!(counts(&backend), (0, 0, 0));
}

#[test]
#[ignore = "requires installed car geometry/textures (set NFSMW_GAME_DIR)"]
fn installed_surface_variants_isolate_actors_combine_glass_and_retire() {
    let mut backend = NullBackend::new(crate::settings::test_caps::native())
        .with_mesh_tracking()
        .with_texture_tracking()
        .with_glossy_tracking()
        .with_normal_tracking();
    let rig = CarRig::upload(&mut backend, keys::next_car(), model(), CarShading::Original, Vec3::Z);
    let pristine = counts(&backend);
    assert!(rig.glass.borrow().is_empty(), "no eager damage cross-product");
    let live = BTreeSet::from([
        SurfaceDamage::default(),
        SurfaceDamage::new(0, 8),
        SurfaceDamage::new(1, 8),
        SurfaceDamage::new(0, 56),
    ]);
    rig.prepare_surfaces(&mut backend, &live, Vec3::Z);
    let prepared = counts(&backend);
    assert!(prepared.0 > pristine.0, "the installed brake/window surfaces require changed uploads");
    assert_eq!((prepared.1, prepared.2), (pristine.1, pristine.2), "reuse the rig's textures/materials");
    assert_eq!(rig.glass.borrow().len(), 3);
    for _ in 0..60 {
        rig.prepare_surfaces(&mut backend, &live, Vec3::Z);
        for lamps in [[false, false], [true, false], [false, true], [true, true]] {
            let intact = draw(&rig, 7, 0, 0, lamps);
            let damaged = draw(&rig, 8, 1, 8, lamps);
            assert!(!intact.is_empty() && !damaged.is_empty());
            let before = rig.swaps().with_lamp_state(lamps[0], lamps[1], &rig.model.textures);
            let after =
                rig.swaps_for([0]).with_broken_brake_lights(8).with_lamp_state(lamps[0], lamps[1], &rig.model.textures);
            assert!(assert_materials(&rig, &backend, 7, &intact, &before) > 0);
            assert!(assert_materials(&rig, &backend, 8, &damaged, &after) > 0);
            let intact_again = draw(&rig, 9, 0, 0, lamps);
            assert_eq!(
                intact.iter().map(|i| i.mesh).collect::<Vec<_>>(),
                intact_again.iter().map(|i| i.mesh).collect::<Vec<_>>()
            );
        }
    }
    assert_eq!(counts(&backend), prepared, "repeated frames/input-state changes add no GPU resources");
    let intact = draw(&rig, 7, 0, 0, [true, true]);
    let left = draw(&rig, 7, 0, 8, [true, true]);
    assert!(intact.iter().zip(&left).any(|(a, b)| a.mesh != b.mesh));
    let window = rig.appearance.borrow().materials.texture(hash("WINDOW_DAMAGE0")).unwrap();
    let combined = draw(&rig, 8, 1, 8, [true, true]);
    assert!(
        combined.iter().any(|i| backend.mesh_draws[&i.mesh].iter().any(|group| group.texture == Some(window))),
        "combined damage retains cracked-window material"
    );
    let variants = rig.glass.borrow();
    let all = &variants[&SurfaceDamage::new(0, 56)];
    assert!(!all.parts.is_empty());
    for part in all.parts.values() {
        assert_eq!(part.lamps[0], part.lamps[2]);
        assert_eq!(part.lamps[1], part.lamps[3], "broken sides deduplicate brake OFF/ON input variants");
    }
    assert!(all.parts.len() < rig.appearance.borrow().parts.len(), "only affected solids are uploaded");
    drop(variants);
    rig.prepare_surfaces(&mut backend, &BTreeSet::from([SurfaceDamage::new(0, 8)]), Vec3::Z);
    assert_eq!(rig.glass.borrow().len(), 1, "unrequested shared keys are retired");
    rig.prepare_surfaces(&mut backend, &BTreeSet::new(), Vec3::Z);
    assert_eq!(counts(&backend), pristine, "repair releases every damage variant");
    let repaired = draw(&rig, 7, 0, 0, [true, true]);
    assert_eq!(repaired.iter().map(|i| i.mesh).collect::<Vec<_>>(), intact.iter().map(|i| i.mesh).collect::<Vec<_>>());
    rig.release(&mut backend);
    assert_eq!(counts(&backend), (0, 0, 0));
}

fn render_surfaces(api: blackbox_gfx::GraphicsApi) {
    use blackbox_gfx::{FrameParams, Projection};
    use blackbox_render::{Renderer, RendererOptions};
    let mut renderer = Renderer::headless(
        (640, 360),
        RendererOptions { backend: api, vsync: false, force_fallback_adapter: false, pipeline_cache_dir: None },
    )
    .unwrap();
    let mut graphics = *renderer.graphics();
    graphics.post = Default::default();
    graphics.ao.quality = blackbox_gfx::AoQuality::Off;
    graphics.shadows = blackbox_gfx::ShadowQuality::Off;
    graphics.reflections = blackbox_gfx::ReflectionQuality::Off;
    renderer.apply_graphics(&graphics);
    let rig = CarRig::upload(&mut renderer, keys::next_car(), model(), CarShading::Original, Vec3::Z);
    rig.prepare_surfaces(
        &mut renderer,
        &BTreeSet::from([SurfaceDamage::new(0, 8), SurfaceDamage::new(0, 56)]),
        Vec3::Z,
    );
    let mut maximum = [0; 2];
    for x in [-6.0, 6.0] {
        let eye = Vec3::new(x, 4.5, 2.4);
        let frame = FrameParams {
            view: glam::camera::rh::view::look_at_mat4(eye, Vec3::Z * 0.7, Vec3::Z),
            projection: Projection::PerspectiveInfiniteReverse {
                fov_y: 42.0_f32.to_radians(),
                aspect: 16.0 / 9.0,
                near: 0.1,
            },
            camera_position: eye,
            light_dir: -Vec3::Z,
            clear_color: [0.08; 3],
            fog: None,
            scattering: None,
            surface_light: None,
            camera_cut: true,
            shadow_tuning: None,
        };
        let mut images = Vec::new();
        for broken in [0, 8, 56] {
            renderer.render(&frame, &draw(&rig, 7, 0, broken, [true, true])).unwrap();
            images.push(renderer.read_output().unwrap().unwrap());
        }
        for (index, broken) in images[1..].iter().enumerate() {
            let changed = images[0]
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .zip(broken.rgba.as_chunks::<4>().0)
                .filter(|(a, b)| a[..3].iter().zip(&b[..3]).any(|(a, b)| a.abs_diff(*b) > 3))
                .count();
            maximum[index] = maximum[index].max(changed);
        }
    }
    eprintln!("{api:?} brake surface changed pixels: {maximum:?}");
    assert!(maximum[0] > 8, "individual left brake OFF needs visible native material output");
    assert!(maximum[1] > 8, "all broken brake surfaces need visible native material output");
    rig.release(&mut renderer);
}

#[test]
#[ignore = "requires installed textures and native Vulkan GPU"]
fn installed_broken_brake_surfaces_render_on_native_vulkan() {
    render_surfaces(blackbox_gfx::GraphicsApi::Vulkan);
}

#[test]
#[ignore = "requires installed textures and native DX12 GPU"]
fn installed_broken_brake_surfaces_render_on_native_dx12() {
    render_surfaces(blackbox_gfx::GraphicsApi::Dx12);
}
