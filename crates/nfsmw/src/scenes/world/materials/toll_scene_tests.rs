//! Actual toll placement through streaming, main-view selection and native rendering.

use super::super::{Options, WorldScene};
use crate::{input::ActionState, viewer::Scene};
use blackbox_gfx::{
    AoQuality, EffectLayer, GraphicsApi, ReflectionQuality, RenderBackend, RenderExperiments, RgbaImage, ShadowQuality,
};
use blackbox_render::{Renderer, RendererOptions};
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};
use std::time::{Duration, Instant};

const SIZE: (u32, u32) = (320, 320);

fn bytes(dir: &GameDir, index: &WorldIndex, name: &str) -> Vec<u8> {
    let section = &index.sections[index.by_name(name).unwrap()];
    dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap()
}

#[test]
#[ignore = "requires installed original world assets and a Vulkan GPU"]
fn installed_toll_placement_streams_selects_and_animates_vulkan() {
    run(GraphicsApi::Vulkan);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "requires installed original world assets and a DX12 GPU"]
fn installed_toll_placement_streams_selects_and_animates_dx12() {
    run(GraphicsApi::Dx12);
}

fn run(api: GraphicsApi) {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    // Scenery-only parsing avoids building unrelated tile vertex buffers for fixture discovery.
    let sections =
        blackbox_scenery::read_scenery_sections(&bytes(&dir, &index, "D51"), &blackbox_scenery::layout::MOST_WANTED)
            .unwrap();
    let placements: Vec<_> = sections
        .iter()
        .flat_map(|s| {
            s.instances.iter().filter_map(move |instance| {
                let info = s.info_of(instance)?;
                info.name.to_ascii_uppercase().contains("TOLLBOOTH_LIGHT").then_some((instance, info))
            })
        })
        .collect();
    assert_eq!(placements.len(), 6, "the selected installed toll section has six authored signal heads");
    let (placement, info) = placements[0];
    let model_hash = info.best_solid().unwrap();
    let solids = blackbox_solid::read_solids_filtered(&bytes(&dir, &index, "X0"), |hash| hash == model_hash).unwrap();
    let solid = solids.first().unwrap();
    let signal_hash = blackbox_hash::bstring_hash("SGN_SIGNAL_CROSS_");
    let group = solid.groups.iter().find(|g| g.diffuse_texture(solid) == Some(signal_hash)).unwrap();
    let indices = &solid.indices[group.first_index as usize..group.first_index as usize + 3];
    let points: [Vec3; 3] = std::array::from_fn(|i| {
        Vec3::from(solid.vertices[(u32::from(indices[i]) + group.base_vertex) as usize].position)
    });
    let transform = Mat4::from_cols_array_2d(&placement.matrix_columns());
    let target = transform.transform_point3((Vec3::from(group.bounds_min) + Vec3::from(group.bounds_max)) * 0.5);
    let normal = transform.transform_vector3((points[1] - points[0]).cross(points[2] - points[0])).normalize();
    let eye = target + normal * 4.0;
    let forward = -normal;
    let mut renderer = Renderer::headless(
        SIZE,
        RendererOptions { backend: api, vsync: false, force_fallback_adapter: false, pipeline_cache_dir: None },
    )
    .expect("selected GPU API must be available");
    let mut graphics = *renderer.graphics();
    graphics.post = Default::default();
    graphics.ao.quality = AoQuality::Off;
    graphics.shadows = ShadowQuality::Off;
    graphics.reflections = ReflectionQuality::Off;
    renderer.apply_graphics(&graphics);
    let mut world = WorldScene::open(
        &dir,
        Options {
            at: Some([eye.x, eye.y]),
            height: 0.0,
            heading: 0.0,
            pitch: 0.0,
            fog_distance: 1500.0,
            wait_for_load: true,
            drive: None,
        },
    )
    .unwrap();
    world.traffic = None;
    world.init(&mut renderer).unwrap();
    world.camera.position = eye;
    world.camera.yaw = forward.y.atan2(forward.x);
    world.camera.pitch = forward.z.asin();
    let start = Instant::now();
    while !world.ready() {
        world.update(&mut renderer, &ActionState::default(), 0.0);
        assert!(start.elapsed() < Duration::from_secs(90), "actual toll residency did not complete");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(world.players.lead_mut().is_none());
    // Ground placement can move a free camera on initial load. Restore the authored face view.
    world.camera.position = eye;
    world.camera.yaw = forward.y.atan2(forward.x);
    world.camera.pitch = forward.z.asin();
    let p = world
        .residency
        .placed()
        .find(|p| p.transform.w_axis.truncate().distance(Vec3::from(placement.position)) < 0.01)
        .expect("the authored toll placement must reach live residency");
    assert!(p.lods.iter().flatten().any(|m| world.residency.shared.meshes.get(&model_hash).is_some_and(|e| e.0 == *m)));
    let key = p.key;
    renderer.set_colour_treatment(None, None);
    for original in [false, true] {
        renderer.set_render_experiments(RenderExperiments { original_world_materials: original, ..Default::default() });
        let mut captures = Vec::new();
        for seconds in [0.0, 0.5, 1.0] {
            world.clock = seconds;
            world.update(&mut renderer, &ActionState::default(), 0.0);
            // This test covers the animated authored surface; separate GPU tests cover flares.
            renderer.set_effects(&EffectLayer::default());
            let (mut frame, instances) = world.frame(1.0);
            assert!(
                instances.iter().any(|i| i.key == key),
                "actual toll head must survive main-view visibility/LOD selection"
            );
            frame.camera_cut = true;
            renderer.render(&frame, instances).unwrap();
            let image = renderer.read_output().expect("headless output").unwrap();
            captures.push(centre(&image));
        }
        let changed =
            captures[0].iter().zip(&captures[1]).filter(|(a, b)| (0..3).any(|c| a[c].abs_diff(b[c]) > 8)).count();
        assert!(
            changed > 20,
            "original_materials={original}: actual toll surface did not visibly change ({changed} pixels)"
        );
        assert_eq!(captures[0], captures[2], "original_materials={original}: the actual toll signal did not loop");
    }
    world.close(&mut renderer);
}

fn centre(image: &RgbaImage) -> Vec<[u8; 4]> {
    // Only the projected centre of the actual signal face can satisfy this assertion.
    let mut pixels = Vec::new();
    for y in image.height / 2 - 16..image.height / 2 + 16 {
        for x in image.width / 2 - 16..image.width / 2 + 16 {
            let offset = ((y * image.width + x) * 4) as usize;
            pixels.push(image.rgba[offset..offset + 4].try_into().unwrap());
        }
    }
    pixels
}
