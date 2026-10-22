//! Installed road/glass rendering, isolated from sky and post-processing changes.
use super::super::{Options, WorldScene, resident::SectionResources};
use crate::{input::ActionState, viewer::Scene};
use blackbox_gfx::{
    AoQuality, FrameParams, Instance, Projection, ReflectionQuality, RenderBackend, RenderExperiments, RgbaImage,
    ShadowQuality,
};
use blackbox_render::{Renderer, RendererOptions};
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::world::{DEFAULT_TRACK, SectionData, WorldIndex};
use std::{
    collections::HashSet,
    path::PathBuf,
    time::{Duration, Instant},
};

const ROAD: &str = "TRN_CT_ROAD_COLONIAL_01_CHOP_H7_R1";
const SHOP: &str = "XB_DONUTSHOP_1B_BK_00";
const SIZE: (u32, u32) = (640, 360);
const EYE: Vec3 = Vec3::new(1945.0, 1850.0, 155.0);

fn load(dir: &GameDir, index: &WorldIndex, name: &str) -> SectionData {
    let i = index.by_name(name).expect("installed fixture section");
    let section = &index.sections[i];
    let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
    nfsmw_data::world::parse_section(i, &bytes).unwrap()
}

fn actual_instance(data: &SectionData, hash: u32, mesh: blackbox_gfx::MeshHandle) -> Instance {
    data.scenery
        .iter()
        .flat_map(|section| {
            section.instances.iter().filter_map(move |instance| {
                let info = section.info_of(instance)?;
                if info.solid_keys.first().copied() != Some(hash) {
                    return None;
                }
                let center = (Vec3::from_array(instance.bbox_min) + Vec3::from_array(instance.bbox_max)) * 0.5;
                Some((center.distance_squared(EYE), Mat4::from_cols_array_2d(&instance.matrix_columns())))
            })
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, transform)| Instance::new(mesh, transform))
        .expect("actual reference placement")
}

fn capture(renderer: &mut Renderer, frame: &FrameParams, instance: Instance, name: &str) -> RgbaImage {
    renderer.render(frame, &[instance]).unwrap();
    let image = renderer.read_output().expect("headless output").unwrap();
    let visible = image.rgba.as_chunks::<4>().0.iter().filter(|p| p[..3].iter().any(|v| *v > 3)).count();
    assert!(visible > 100, "{name}: actual surface must cover visible pixels, got {visible}");
    if let Some(folder) = std::env::var_os("NFSMW_SURFACE_CAPTURE_DIR") {
        let folder = PathBuf::from(folder);
        std::fs::create_dir_all(&folder).unwrap();
        let api = renderer.info().api.to_string();
        let path = folder.join(format!("{api}-{name}.png"));
        let file = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
        let mut encoder = png::Encoder::new(file, image.width, image.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
    }
    image
}

fn assert_surface_changed(a: &RgbaImage, b: &RgbaImage, reason: &str) {
    let mut compared = 0;
    let mut changed = 0;
    for (a, b) in a.rgba.as_chunks::<4>().0.iter().zip(b.rgba.as_chunks::<4>().0) {
        // Both captures contain only this actual surface against black. Excluding
        // black pixels prevents clear/sky changes satisfying the lighting check.
        if a[..3].iter().all(|v| *v <= 3) || b[..3].iter().all(|v| *v <= 3) {
            continue;
        }
        compared += 1;
        changed += usize::from((0..3).any(|c| a[c].abs_diff(b[c]) > 2));
    }
    assert!(compared > 100, "{reason}: too little common visible surface");
    assert!(
        changed > 20 && changed * 100 > compared,
        "{reason}: only {changed}/{compared} common surface pixels changed"
    );
}

fn view(mut frame: FrameParams, target: Vec3) -> FrameParams {
    frame.view = crate::viewer::camera::Lens::look_at(EYE, target, 55.0, 0.5, SIZE.0 as f32 / SIZE.1 as f32).view;
    frame.projection = Projection::PerspectiveInfiniteReverse {
        fov_y: 55.0_f32.to_radians(),
        aspect: SIZE.0 as f32 / SIZE.1 as f32,
        near: 0.5,
    };
    frame.camera_position = EYE;
    frame.clear_color = [0.0; 3];
    frame.fog = None;
    frame.camera_cut = true;
    frame
}

fn lighting_snapshot(world: &WorldScene) -> Vec<f32> {
    let light = world.surface_light().expect("installed original world light");
    let fog = world.scattering().expect("installed original scattering");
    let mut values = Vec::new();
    values.extend(light.ambient);
    values.extend(light.diffuse);
    values.extend(light.specular);
    values.push(light.shadow_strength);
    values.extend([light.ambient_alpha, light.specular_power, light.reflection_scale, light.reflective_specular_scale]);
    values.extend(fog.rayleigh);
    values.extend(fog.mie);
    values.extend(fog.to_sun.to_array());
    values.extend([fog.g, fog.distance_scale, fog.in_scatter_scale, world.day.hour(), world.atmosphere.intensity()]);
    values
}

fn assert_actual_road_shadows(renderer: &mut Renderer, frame: &FrameParams, road: Instance) {
    use blackbox_gfx::{BlendMode, DrawRange, MeshDesc, Shading, Vertex};
    let vertices = [[1926.0, 1846.0, 153.0], [1940.0, 1846.0, 153.0], [1940.0, 1854.0, 153.0], [1926.0, 1854.0, 153.0]]
        .map(|position| Vertex { position, normal: [0.0, 0.0, 1.0], color_bgra: [255; 4], uv: [0.0; 2] });
    let caster = renderer.create_mesh(&MeshDesc {
        label: "isolated installed road occluder",
        vertices: &vertices,
        secondary_uvs: None,
        tangents: None,
        indices: &[0, 1, 2, 0, 2, 3],
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: None,
            blend: BlendMode::Opaque,
            shading: Shading::Prelit,
            two_sided: true,
        }],
    });
    let mut frame = *frame;
    frame.light_dir = Vec3::new(0.2, 0.3, -1.0).normalize();
    frame.surface_light.as_mut().unwrap().shadow_strength = 0.9;
    let baseline = RenderExperiments { original_world_materials: true, ..Default::default() };
    renderer.set_render_experiments(baseline);
    let off = capture(renderer, &frame, road, "road-shadow-off");
    let mut casters = [Instance::new(caster, Mat4::IDENTITY)];
    for (name, projection) in [("custom", None), ("projected", Some(crate::settings::appearance::shadows::PROJECTION))]
    {
        casters[0].transform = Mat4::IDENTITY;
        renderer.set_shadow_casters(Some(&casters));
        renderer.set_render_experiments(RenderExperiments {
            sun_shadows: true,
            shadow_resolution: 2048,
            shadow_projection: projection,
            ..baseline
        });
        let on = capture(renderer, &frame, road, &format!("road-shadow-{name}"));
        let dark = off
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .zip(on.rgba.as_chunks::<4>().0)
            .filter(|(a, b)| a[0] > b[0].saturating_add(3))
            .count();
        assert!(dark > 20, "{name}: actual effect-1 road must receive shadows, {dark} darkened pixels");
        casters[0].transform = Mat4::from_translation(Vec3::new(-4.0, 0.0, 0.0));
        renderer.set_shadow_casters(Some(&casters));
        let moved = capture(renderer, &frame, road, &format!("road-shadow-{name}-moved"));
        assert_surface_changed(&on, &moved, &format!("{name} caster movement"));
        renderer.set_render_experiments(baseline);
        let restored = capture(renderer, &frame, road, &format!("road-shadow-{name}-off"));
        assert_eq!(off.rgba, restored.rgba, "shadow toggle must restore original road exactly");
    }
    renderer.set_shadow_casters(None);
    renderer.destroy_mesh(caster);
}

fn assert_high_fps_lighting(world: &mut WorldScene, dir: &GameDir) {
    let metadata = nfsmw_data::read_unwrapped(dir, &format!("TRACKS/{DEFAULT_TRACK}.BUN")).unwrap();
    world.set_rain(true, 1.0);
    let mut reference: Option<Vec<f32>> = None;
    for fps in [30, 60, 144, 360] {
        world.atmosphere = super::super::lighting::Clouds::read(&metadata).unwrap();
        world.configure_day(crate::viewer::Environment { hour: 16.0, seconds_per_hour: 10.0, ..Default::default() });
        for _ in 0..fps * 2 {
            let dt = 1.0 / fps as f32;
            world.update_atmosphere(dt);
            world.update_day(dt, false);
        }
        let values = lighting_snapshot(world);
        assert!(world.atmosphere.intensity() > 0.1, "the FPS test must exercise changing clouds");
        if let Some(reference) = &reference {
            for (index, (a, b)) in values.iter().zip(reference).enumerate() {
                assert!((a - b).abs() < 0.00002, "{fps} FPS light input {index}: {a} vs {b}");
            }
        }
        if reference.is_none() {
            reference = Some(values);
        }
    }
    world.set_rain(false, 0.0);
}

#[test]
#[ignore = "needs installed original world assets and a Vulkan GPU"]
fn installed_original_road_and_glass_render_with_live_lighting_at_high_fps_vulkan() {
    installed_surface_run(blackbox_gfx::GraphicsApi::Vulkan);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "needs installed original world assets and a DX12 GPU"]
fn installed_original_road_and_glass_render_with_live_lighting_at_high_fps_dx12() {
    installed_surface_run(blackbox_gfx::GraphicsApi::Dx12);
}

fn installed_surface_run(api: blackbox_gfx::GraphicsApi) {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
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
    renderer.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    let mut world = WorldScene::open(
        &dir,
        Options {
            at: Some([EYE.x, EYE.y]),
            height: 5.0,
            heading: 180.0,
            pitch: -8.0,
            fog_distance: 1500.0,
            wait_for_load: true,
            drive: None,
        },
    )
    .unwrap();
    world.traffic = None;
    world.init(&mut renderer).unwrap();
    let start = Instant::now();
    loop {
        world.update(&mut renderer, &ActionState::default(), 0.0);
        if world.ready() {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(90), "installed residency did not complete");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(world.residency.complete());
    assert_high_fps_lighting(&mut world, &dir);
    renderer.set_colour_treatment(None, None);

    // Upload selected actual groups with the real material resolver. Restricting
    // geometry makes these assertions about road and glass, not moving sky pixels.
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut tile = load(&dir, &index, "H47");
    let mut shared_data = load(&dir, &index, "X0");
    tile.solids.retain(|s| s.name == ROAD);
    shared_data.solids.retain(|s| s.name == SHOP);
    assert_eq!((tile.solids.len(), shared_data.solids.len()), (1, 1));
    tile.solids[0].groups = vec![tile.solids[0].groups[2].clone()];
    shared_data.solids[0].groups.retain(|g| g.effect_id == 5);
    assert_eq!(tile.solids[0].groups[0].effect_id, 1);
    assert_eq!(shared_data.solids[0].groups.len(), 2);
    let mut needed: HashSet<_> =
        tile.solids[0].texture_hashes.iter().chain(&shared_data.solids[0].texture_hashes).copied().collect();
    needed.insert(blackbox_hash::bstring_hash("WINDOWREFLECTION"));
    let mut textures = load(&dir, &index, "Y0").textures;
    let bytes = nfsmw_data::read_unwrapped(&dir, "GLOBAL/INGAMEA.BUN").unwrap();
    textures.extend(blackbox_tpk::read_texture_packs(&bytes).unwrap().into_iter().flat_map(|p| p.textures));
    textures.retain(|t| needed.contains(&t.name_hash));
    assert!(needed.iter().all(|h| textures.iter().any(|t| t.name_hash == *h)));
    let mut shared = SectionResources::default();
    shared.upload_texture_list(&mut renderer, &textures, &[]);
    shared.upload_solid_list(&mut renderer, &shared_data.solids, None, &SectionResources::default(), false);
    let mut local = SectionResources::default();
    local.upload_solid_list(&mut renderer, &tile.solids, None, &shared, false);
    assert!(!local.normal_materials.is_empty(), "actual road needs registered original material");
    assert!(!shared.window_materials.is_empty(), "actual glass needs registered original material");
    let road = actual_instance(&tile, tile.solids[0].name_hash, local.meshes[&tile.solids[0].name_hash].0);
    let window_placements = load(&dir, &index, "H61");
    let window = actual_instance(
        &window_placements,
        shared_data.solids[0].name_hash,
        shared.meshes[&shared_data.solids[0].name_hash].0,
    );
    for (name, instance, target) in
        [("road", road, Vec3::new(1930.0, 1850.0, 149.8)), ("window", window, Vec3::new(1907.0, 1858.0, 154.5))]
    {
        world.configure_day(crate::viewer::Environment { hour: 12.0, seconds_per_hour: 150.0, ..Default::default() });
        let noon = view(world.frame(SIZE.0 as f32 / SIZE.1 as f32).0, target);
        renderer.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
        let original = capture(&mut renderer, &noon, instance, &format!("{name}-original-noon"));
        if name == "road" {
            assert_actual_road_shadows(&mut renderer, &noon, instance);
        }
        renderer.set_render_experiments(RenderExperiments::default());
        let legacy = capture(&mut renderer, &noon, instance, &format!("{name}-legacy-noon"));
        assert_surface_changed(&original, &legacy, &format!("{name} original material selection"));
        renderer.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
        world.configure_day(crate::viewer::Environment { hour: 18.0, seconds_per_hour: 150.0, ..Default::default() });
        let dusk = view(world.frame(SIZE.0 as f32 / SIZE.1 as f32).0, target);
        let changed = capture(&mut renderer, &dusk, instance, &format!("{name}-original-dusk"));
        assert_surface_changed(&original, &changed, &format!("{name} authored lighting change"));
        assert!(noon.surface_light != dusk.surface_light && noon.light_dir != dusk.light_dir);
    }
    local.release(&mut renderer);
    shared.release(&mut renderer);
    world.close(&mut renderer);
}
