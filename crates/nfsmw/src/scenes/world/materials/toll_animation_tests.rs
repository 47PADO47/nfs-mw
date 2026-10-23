//! Installed toll signal surface through the real mesh, animation and GPU paths.

use super::super::resident::SectionResources;
use blackbox_gfx::{
    AoQuality, FrameParams, GraphicsApi, Instance, Projection, ReflectionQuality, RenderBackend, RenderExperiments,
    ShadowQuality,
};
use blackbox_render::{Renderer, RendererOptions};
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::world::{DEFAULT_TRACK, SectionData, WorldIndex};

fn load(dir: &GameDir, index: &WorldIndex, name: &str) -> SectionData {
    let i = index.by_name(name).expect("installed shared section");
    let section = &index.sections[i];
    let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
    nfsmw_data::world::parse_section(i, &bytes).unwrap()
}

#[test]
#[ignore = "requires installed original assets and a Vulkan GPU"]
fn installed_toll_signal_animation_changes_surface_and_loops_vulkan() {
    run(GraphicsApi::Vulkan);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "requires installed original assets and a DX12 GPU"]
fn installed_toll_signal_animation_changes_surface_and_loops_dx12() {
    run(GraphicsApi::Dx12);
}

fn run(api: GraphicsApi) {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut geometry = load(&dir, &index, "X0");
    let textures = load(&dir, &index, "Y0");
    let solid = geometry.solids.iter_mut().find(|s| s.name == "XO_TOLLBOOTH_LIGHT_1B_00").unwrap();
    let hash = blackbox_hash::bstring_hash("SGN_SIGNAL_CROSS_");
    let animation = textures.anims.iter().find(|a| a.name_hash == hash).unwrap().clone();
    assert_eq!((animation.frames_per_second, animation.frames.len()), (2, 2));
    assert_eq!(animation.frames[0], hash);
    let signal_slot = solid.texture_hashes.iter().position(|h| *h == hash).unwrap();
    solid.groups.retain(|g| usize::from(g.texture_slots[0]) == signal_slot);
    assert_eq!(solid.groups.len(), 1, "isolate the installed animated signal face");
    assert_eq!(solid.groups[0].effect_id, 0);
    let group = &solid.groups[0];
    let triangle = &solid.indices[group.first_index as usize..group.first_index as usize + 3];
    let points: [Vec3; 3] = std::array::from_fn(|i| {
        Vec3::from(solid.vertices[(u32::from(triangle[i]) + group.base_vertex) as usize].position)
    });
    let center = (Vec3::from(group.bounds_min) + Vec3::from(group.bounds_max)) * 0.5;
    let normal = (points[1] - points[0]).cross(points[2] - points[0]).normalize();
    let eye = center + normal * 3.0;
    let mut renderer = Renderer::headless(
        (256, 256),
        RendererOptions { backend: api, vsync: false, force_fallback_adapter: false, pipeline_cache_dir: None },
    )
    .expect("selected GPU API must be available");
    let mut graphics = *renderer.graphics();
    graphics.post = Default::default();
    graphics.ao.quality = AoQuality::Off;
    graphics.shadows = ShadowQuality::Off;
    graphics.reflections = ReflectionQuality::Off;
    renderer.apply_graphics(&graphics);
    renderer.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    let mut resources = SectionResources::default();
    let needed: Vec<_> = textures.textures.into_iter().filter(|t| animation.frames.contains(&t.name_hash)).collect();
    assert_eq!(needed.len(), 2, "both installed animation frames must be resident");
    resources.upload_texture_list(&mut renderer, &needed, &[]);
    resources.upload_solid_list(&mut renderer, std::slice::from_ref(solid), &SectionResources::default(), false);
    resources.anims.push(animation);
    let instance = Instance::new(resources.meshes[&solid.name_hash].0, Mat4::IDENTITY);
    let frame = FrameParams {
        view: crate::viewer::camera::Lens::look_at(eye, center, 45.0, 0.1, 1.0).view,
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 45_f32.to_radians(), aspect: 1.0, near: 0.1 },
        camera_position: eye,
        light_dir: -normal,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: true,
        shadow_tuning: None,
    };
    let mut capture = |seconds| {
        resources.animate(&mut renderer, seconds, &SectionResources::default());
        renderer.render(&frame, &[instance]).unwrap();
        renderer.read_output().expect("headless output").unwrap()
    };
    let first = capture(0.0);
    let next = capture(0.5);
    let looped = capture(1.0);
    let visible = first.rgba.as_chunks::<4>().0.iter().filter(|p| p[..3].iter().any(|v| *v > 8)).count();
    assert!(visible > 100, "the actual signal face must be visible, got {visible} pixels");
    let changed = first
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(next.rgba.as_chunks::<4>().0)
        .filter(|(a, b)| (0..3).any(|c| a[c].abs_diff(b[c]) > 8))
        .count();
    assert!(changed > 100, "the installed half-second frame must change the surface, got {changed} pixels");
    assert_eq!(first.rgba, looped.rgba, "the installed one-second loop must restore the first frame");
    resources.release(&mut renderer);
}
