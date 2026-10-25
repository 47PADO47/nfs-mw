//! With the install (`NFSMW_GAME_DIR`): the HUD package has every object the binding looks for.

use blackbox_feng::{Runtime, fe_hash_upper};
use game_install::GameDir;

use super::bind::HudBinding;
use super::minimap::MinimapBinding;
use crate::ui::{Catalog, UiAssets};

#[test]
#[ignore = "needs installed mirror artwork and HUD package"]
fn installed_mirror_picture_uses_glass_instead_of_the_detector_and_texture_padding() {
    use crate::gui::UiOutput;
    use crate::ui::present::{BlackboxPresenter, Screen, blackbox::MIRROR_TEXTURE};
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let assets = UiAssets::load(&dir).unwrap();
    let mut rt = crate::ui::runtime();
    let id = rt.load(catalog.find("HUD_SingleRace.fng").unwrap().clone());
    let mut binding = HudBinding::new(&mut rt, id);
    binding.advance_mirror_fade(1.0);
    binding.apply(&mut rt, &super::state::HudState::default());
    rt.update(1.0);
    binding.finish_frame(&mut rt, &super::state::HudState::default());
    let mut presenter = BlackboxPresenter::default();
    let mut out = UiOutput::default();
    presenter.present(&rt.tree(id), &assets, Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 }, &mut out);
    let mesh = out.layer.unwrap().meshes.into_iter().find(|m| m.texture == MIRROR_TEXTURE).expect("mirror glass");
    let mut lo = glam::Vec2::splat(f32::MAX);
    let mut hi = glam::Vec2::splat(f32::MIN);
    for v in &mesh.vertices {
        let p = glam::Vec2::from_array(v.position);
        lo = lo.min(p);
        hi = hi.max(p);
        assert!((0.0..=1.0).contains(&v.uv[0]) && (0.0..=1.0).contains(&v.uv[1]));
    }
    let size = hi - lo;
    assert!(mesh.vertices.len() > 100, "the curved outline uses the artwork's rows");
    assert!(size.x / size.y > 3.0 && size.x / size.y < 5.0, "glass aspect: {size:?}");
    assert!((lo.x + hi.x - 1920.0).abs() < 3.0, "mirror stays centered: {lo:?} {hi:?}");
    eprintln!("installed mirror glass: {lo:?} to {hi:?}, {} vertices", mesh.vertices.len());
}

#[test]
#[ignore = "needs the game; validates Speedbreaker presenter meshes and screen placement"]
fn installed_speedbreaker_presenter_draws_bottom_central_pulse_and_hides_normal_gauges() {
    use super::state::{HudState, MapPosition};
    use super::viewport::HudViewport;
    use crate::gui::UiOutput;
    use crate::settings::HudLayout;
    use crate::ui::present::{BlackboxPresenter, Screen};
    use blackbox_feng::NodeKind;
    use blackbox_gfx::UiTextureId;
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let assets = UiAssets::load(&dir).unwrap();
    let mut rt = crate::ui::runtime();
    let id = rt.load(catalog.find("HUD_SingleRace.fng").unwrap().clone());
    let minimap = MinimapBinding::open_city(&rt, id, &dir, &assets).unwrap();
    let mut binding = HudBinding::new(&mut rt, id).with_minimap(Some(minimap));
    let viewport = HudViewport::new(&rt.tree(id), &[]);
    let state = HudState {
        speedbreaker: 0.7,
        speedbreaker_active: true,
        minimap: Some(MapPosition {
            position: [1925.0, 1874.0],
            heading: [1.0, 0.0],
            orientation: blackbox_minimap::Orientation::North,
        }),
        ..Default::default()
    };
    for _ in 0..120 {
        binding.apply(&mut rt, &state);
        rt.update(1.0 / 60.0);
    }
    let tree = rt.tree(id);
    let pulse = tree.nodes.iter().find(|node| node.name_hash == 0xFC46_5765).unwrap();
    let NodeKind::Image { texture, .. } = pulse.kind else { panic!("central stopwatch is an image") };
    let texture_id = UiTextureId((1 << 62) | u64::from(assets.resolve(texture)));
    let absent_texture_ids: Vec<_> = ["TAC_Lines_7500", "PLAYERCARINDICATOR"]
        .into_iter()
        .map(|name| {
            let node = tree.nodes.iter().find(|node| node.name_hash == fe_hash_upper(name)).unwrap();
            let NodeKind::Image { texture, .. } = node.kind else { panic!("normal HUD element is an image") };
            UiTextureId((1 << 62) | u64::from(assets.resolve(texture)))
        })
        .collect();
    for (width, height) in [(1280.0, 720.0), (1440.0, 1080.0), (2560.0, 1080.0)] {
        for layout in [HudLayout::Pc, HudLayout::Classic, HudLayout::Xbox360] {
            let screen = Screen { width, height, pixels_per_point: 1.0 };
            let mut tree = rt.tree(id);
            viewport.apply(&mut tree, screen, layout);
            let mut out = UiOutput::default();
            BlackboxPresenter::default().present(&tree, &assets, screen, &mut out);
            let layer = out.layer.unwrap();
            assert!(
                layer.meshes.iter().all(|mesh| !absent_texture_ids.contains(&mesh.texture)),
                "presenter emits neither tachometer face nor minimap player arrow while active"
            );
            let vertices: Vec<_> =
                layer.meshes.iter().filter(|mesh| mesh.texture == texture_id).flat_map(|mesh| &mesh.vertices).collect();
            assert!(!vertices.is_empty(), "actual central pulse mesh is emitted");
            assert!(
                vertices.iter().any(|vertex| vertex.color_rgba[..3].iter().any(|channel| *channel > 0)),
                "pulse vertices carry visible colour"
            );
            let left = vertices.iter().map(|vertex| vertex.position[0]).fold(f32::INFINITY, f32::min);
            let right = vertices.iter().map(|vertex| vertex.position[0]).fold(f32::NEG_INFINITY, f32::max);
            let top = vertices.iter().map(|vertex| vertex.position[1]).fold(f32::INFINITY, f32::min);
            let bottom = vertices.iter().map(|vertex| vertex.position[1]).fold(f32::NEG_INFINITY, f32::max);
            assert!(((left + right) * 0.5 / width - 0.5).abs() < 0.05, "central pulse horizontally centered");
            assert!(top > height * 0.7 && bottom < height, "central pulse is at the bottom, not mirror position");
            let gauge = tree.nodes.iter().find(|node| node.name_hash == fe_hash_upper("GaugeCluster")).unwrap();
            assert!(!gauge.visible || gauge.world_colour[3] == 0, "normal gauge cannot emit visible vertices");
        }
    }
}

#[test]
#[ignore = "needs the game; validates authored Speedbreaker active and exit layout"]
fn installed_speedbreaker_active_layout_replaces_normal_gauges_and_restores_them() {
    use super::state::HudState;
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    for fps in [5, 60, 144, 360] {
        let mut rt = crate::ui::runtime();
        let id = rt.load(catalog.find("HUD_SingleRace.fng").unwrap().clone());
        let bar = rt.find(id, super::speedbreaker::LINEAR_BAR).unwrap();
        let full_width = rt.size(bar).unwrap().x;
        let mut binding = HudBinding::new(&mut rt, id);
        let mut state = HudState { speedbreaker: 0.7, ..Default::default() };
        let step = |rt: &mut Runtime, binding: &mut HudBinding, state: &HudState, count: usize| {
            for _ in 0..count {
                binding.apply(rt, state);
                rt.update(1.0 / fps as f32);
            }
        };
        let alpha = |rt: &Runtime, hash: u32| {
            let tree = rt.tree(id);
            let node = tree.nodes.iter().find(|node| node.name_hash == hash).unwrap();
            match node.visible {
                true => node.world_colour[3],
                false => 0,
            }
        };
        step(&mut rt, &mut binding, &state, fps * 2);
        let gauge = fe_hash_upper("GaugeCluster");
        assert!(alpha(&rt, gauge) > 0, "inactive gauge at {fps} FPS");
        state.speedbreaker_active = true;
        let mut fading_out = false;
        for _ in 0..fps * 2 {
            step(&mut rt, &mut binding, &state, 1);
            fading_out |= (1..255).contains(&alpha(&rt, gauge));
        }
        assert!(fading_out, "authored fade has intermediate opacity at {fps}");
        let group = rt.find(id, super::speedbreaker::LINEAR_GROUP).unwrap();
        assert_eq!(rt.script_of(group), Some(0x5B0D_9106), "installed ACTIVATE response at {fps}");
        assert!(alpha(&rt, super::speedbreaker::LINEAR_GROUP) > 0, "active central bar at {fps}");
        assert_eq!(alpha(&rt, gauge), 0, "normal gauge fades out at {fps}");
        assert!((rt.size(bar).unwrap().x - full_width * 0.7).abs() < 1e-4);
        let pulse_hash = 0xFC46_5765;
        let pulse = rt.find(id, pulse_hash).unwrap();
        assert_eq!(rt.script_of(pulse), Some(0x0382_6A28), "authored looping icon script");
        let mut widths = Vec::new();
        for _ in 0..fps {
            step(&mut rt, &mut binding, &state, 1);
            let tree = rt.tree(id);
            let icon = tree.nodes.iter().find(|node| node.index == pulse.index).unwrap();
            assert!(icon.visible && icon.world_colour[3] > 0, "active pulse icon visible at {fps}");
            widths.push(icon.size.x);
        }
        let low = widths.iter().copied().fold(f32::INFINITY, f32::min);
        let high = widths.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert!(high - low > 1.0, "installed icon pulses in size at {fps}");
        state.speedbreaker_active = false;
        let mut fading_in = false;
        for _ in 0..fps * 2 {
            step(&mut rt, &mut binding, &state, 1);
            fading_in |= (1..255).contains(&alpha(&rt, gauge));
        }
        assert!(fading_in, "authored restoration has intermediate opacity at {fps}");
        assert!(alpha(&rt, gauge) > 0, "normal gauge restored at {fps}");
        assert_eq!(alpha(&rt, super::speedbreaker::LINEAR_GROUP), 0, "central bar exits at {fps}");
        state.speedbreaker_active = true;
        step(&mut rt, &mut binding, &state, 1);
        state.speedbreaker_active = false;
        step(&mut rt, &mut binding, &state, 1);
        state.speedbreaker_active = true;
        step(&mut rt, &mut binding, &state, fps * 2);
        assert!(alpha(&rt, super::speedbreaker::LINEAR_GROUP) > 0, "re-entry central bar at {fps}");
        assert_eq!(alpha(&rt, gauge), 0, "re-entry normal gauge at {fps}");
    }
}

#[test]
#[ignore = "needs the game; validates default KPH Speedbreaker presentation"]
fn installed_speedbreaker_is_visible_and_drives_the_default_kph_gauge() {
    use super::state::HudState;
    use super::viewport::HudViewport;
    use blackbox_feng::NodeKind;
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let mut rt = Runtime::new();
    let id = rt.load(catalog.find("HUD_SingleRace.fng").unwrap().clone());
    let mut binding = HudBinding::new(&mut rt, id);
    let icon = rt.find(id, fe_hash_upper(super::speedbreaker::ICON)).unwrap();
    let viewport = HudViewport::new(&rt.tree(id), &[]);
    for charge in [1.0, 0.3, 0.1, 0.0] {
        let state = HudState { speedbreaker: charge, ..Default::default() };
        assert!(!state.use_mph, "default HUD uses KPH");
        binding.apply(&mut rt, &state);
        rt.update(0.1);
        let mut tree = rt.tree(id);
        viewport.apply(
            &mut tree,
            crate::ui::present::Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 },
            crate::settings::HudLayout::default(),
        );
        for name in [super::speedbreaker::GROUP, super::speedbreaker::MASK] {
            let node = tree.nodes.iter().find(|n| n.name_hash == fe_hash_upper(name)).unwrap();
            assert!(node.visible && node.colour[3] > 0, "{name} must be visible at charge{charge}");
            assert!(node.position.x.abs() < 960.0 && node.position.y.abs() < 540.0, "{name} in viewport");
        }
        let mask = tree.nodes.iter().find(|n| n.name_hash == fe_hash_upper(super::speedbreaker::MASK)).unwrap();
        let NodeKind::Image { mask_rotation, .. } = mask.kind else { panic!("meter mask is an image") };
        assert!((mask_rotation[2] - 175.0 * (1.0 - charge)).abs() < 1e-4);
        let expected = match charge > 0.0 {
            true => 0x7703_1C70,
            false => blackbox_feng::runtime::INIT_SCRIPT,
        };
        assert_eq!(rt.script_of(icon), Some(expected));
        if charge > 0.0 {
            let node = tree.nodes.iter().find(|n| n.index == icon.index).unwrap();
            assert!(node.visible && node.colour[3] > 0, "charged icon visible");
        }
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_single_race_hud_has_the_objects_the_binding_drives() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let package = catalog.find("HUD_SingleRace.fng").expect("HUD_SingleRace.fng").clone();
    let mut rt = Runtime::new();
    let id = rt.load(package);
    let by_name = |n: &str| rt.find(id, fe_hash_upper(n)).is_some();
    for name in [
        "SpeedometerGroup",
        "SPEED_DIGIT_1",
        "SPEED_DIGIT_2",
        "SPEED_DIGIT_3",
        "3rdPersonSpeedUnits",
        "GaugeCluster",
        "3rdPersonGear",
        "3rdPersonNeedle",
        "Shift_light",
        "TAC_Lines_7500",
        "RPM_REDLINE",
        "TURBO_GROUP",
        "3rdperson_TurboDial",
        "Speedbreaker_Meter_Group",
        "SPEEDBREAKER_METER_ICON_GROUP",
        "Speedbreaker_Meter_Multi_Image",
    ] {
        assert!(by_name(name), "{name} is missing");
    }
    for hash in [0x87c3_8e97u32, 0x27dd_f583, 0xedfb_6d37] {
        assert!(rt.find(id, hash).is_some(), "{hash:#x} is missing");
    }
    // Binding every object works.
    let _ = HudBinding::new(&mut rt, id);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_minimap_finds_its_objects_and_its_tiles_in_the_install() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let package = catalog.find("HUD_SingleRace.fng").expect("HUD_SingleRace.fng").clone();
    let mut rt = Runtime::new();
    let id = rt.load(package);
    let assets = UiAssets::load(&dir).unwrap();
    assert!(MinimapBinding::open_city(&rt, id, &dir, &assets).is_some());
    // The textures the pieces' mask and the arrow are drawn with.
    for name in ["MINIMAP_MASK", "MINIMAP_ICON_CAR", "MINIMAP_BACKING_COLOR", "MINI_MAP_CHOP0", "MINI_MAP_CHOP63"] {
        assert!(assets.texture(fe_hash_upper(name)).is_some(), "{name} is missing");
    }
    assert!(rt.find(id, fe_hash_upper("TRACKMAPTARGETRING")).is_some());
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn a_race_event_has_route_tiles_the_minimap_switches_to_and_leaves() {
    use super::event_tiles::{EventTiles, install_loader};
    use super::map_source::{MapSource, resolve_stem};
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let mut rt = Runtime::new();
    let id = rt.load(catalog.find("HUD_SingleRace.fng").expect("HUD_SingleRace.fng").clone());
    let assets = UiAssets::load(&dir).unwrap();
    let mut binding = MinimapBinding::open_city(&rt, id, &dir, &assets).unwrap();
    let mut tiles = EventTiles::new(install_loader(dir.clone()));
    let wait = |tiles: &mut EventTiles, event: Option<&str>| {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while std::time::Instant::now() < until {
            tiles.update(event, &assets);
            if tiles.ready_stem().is_some() || event.is_none() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    };
    let all = |_| true;

    // The reversed route of 10.4.2 has its own file; the pieces use its tiles once the file is read.
    let source = MapSource::Event("10.4.2.r".into());
    tiles.update(source.event(), &assets);
    let before = resolve_stem(&source, Some(14), all, tiles.ready_stem()).to_owned();
    wait(&mut tiles, source.event());
    let stem = resolve_stem(&source, Some(14), all, tiles.ready_stem());
    assert_eq!(stem, "MINI_MAP_10_4_2_R");
    assert!(before == stem || before == "MINI_MAP_UNLOCK_1", "the stage map shows while the file is read");
    assert!(binding.set_stem(stem));
    assert!(binding.resources_available(|key| assets.texture(key).is_some(), |_| true), "all 64 tiles are known");

    // A pursuit challenge has no file: the stage's map stays and nothing is added.
    let challenge = MapSource::Event("19.8.1".into());
    for _ in 0..50 {
        tiles.update(challenge.event(), &assets);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(tiles.ready_stem(), None);
    assert_eq!(resolve_stem(&challenge, Some(14), all, tiles.ready_stem()), "MINI_MAP_UNLOCK_1");

    // The race is over: the tiles are dropped.
    wait(&mut tiles, None);
    assert_eq!(assets.late_count(), 0);
    assert!(assets.texture(fe_hash_upper("MINI_MAP_10_4_2_R_CHOP0")).is_none());
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_pursuit_board_and_cooldown_bar_are_objects_of_the_single_race_hud() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let package = catalog.find("HUD_SingleRace.fng").expect("HUD_SingleRace.fng").clone();
    let mut rt = Runtime::new();
    let id = rt.load(package);
    // The pursuit board, its timer, the bounty row and the heat meter; the cool-down bar and its fill (BasePoly).
    for name in ["PursuitBoardGroup", "PursuitTime", "Cooldown_Meter_Group", "Heat_Meter_Group"] {
        assert!(rt.find(id, fe_hash_upper(name)).is_some(), "{name} is missing");
    }
    for hash in [0x8674_e6d4u32, 0xcf81_7638, 0x60b4_5523, 0x3807_4901] {
        assert!(rt.find(id, hash).is_some(), "{hash:#x} is missing");
    }
    // The bar's fill has the width the binding sets it to.
    let bar = rt.find(id, 0xcf81_7638).unwrap();
    assert!(rt.size(bar).is_some_and(|s| s.x > 0.0), "the cool-down fill has a width");
    let _ = HudBinding::new(&mut rt, id);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR); reports presenter timing"]
fn the_minimap_reuses_stationary_pixels_and_bounds_its_moving_cache() {
    use std::time::Instant;

    use blackbox_minimap::Orientation;

    use super::state::{HudState, MapPosition};
    use crate::gui::UiOutput;
    use crate::ui::present::{BlackboxPresenter, Screen};

    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let assets = UiAssets::load(&dir).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let mut rt = Runtime::new();
    let id = rt.load(catalog.find("HUD_SingleRace.fng").unwrap().clone());
    let minimap = MinimapBinding::open_city(&rt, id, &dir, &assets).expect("complete map");
    let mut binding = HudBinding::new(&mut rt, id).with_minimap(Some(minimap));
    let map = nfsmw_data::minimap::open_city_calibration(&dir).unwrap();
    let screen = Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 };
    let mut presenter = BlackboxPresenter::default();
    let mut s = HudState {
        minimap: Some(MapPosition { position: [2000.0, 1500.0], heading: [0.0, 1.0], orientation: Orientation::North }),
        ..Default::default()
    };
    binding.apply(&mut rt, &s);
    let mut first = UiOutput::default();
    presenter.present(&rt.tree(id), &assets, screen, &mut first);
    assert!(!first.patches.is_empty());
    let slots = presenter.cache_sizes().1;
    for _ in 0..10 {
        let mut out = UiOutput::default();
        presenter.present(&rt.tree(id), &assets, screen, &mut out);
        assert!(out.patches.is_empty(), "stationary HUD must not upload the same pixels again");
    }

    // Visit each tile repeatedly, with both orientations and continuously changing scroll/heading.
    // The cache must not grow with frames, scroll values, or headings.
    const FRAMES: usize = 512;
    let mut bytes = 0usize;
    let started = Instant::now();
    for frame in 0..FRAMES {
        let cell = frame % 64;
        let u = (cell % 8) as f32 / 8.0 + 0.03125 + (frame / 64) as f32 * 0.003;
        let v = (cell / 8) as f32 / 8.0 + 0.03125;
        let angle = frame as f32 * 0.07;
        s.minimap = Some(MapPosition {
            position: [map.origin[0] + u * map.width, map.origin[1] + (1.0 - v) * map.width],
            heading: [angle.sin(), angle.cos()],
            orientation: match frame % 2 {
                0 => Orientation::North,
                _ => Orientation::Heading,
            },
        });
        binding.apply(&mut rt, &s);
        let mut out = UiOutput::default();
        presenter.present(&rt.tree(id), &assets, screen, &mut out);
        bytes += out.patches.iter().map(|p| p.rgba.len()).sum::<usize>();
        assert_eq!(presenter.cache_sizes().1, slots, "moving map must reuse the same GPU slots");
    }
    let elapsed = started.elapsed();
    let (_, masked, decoded) = presenter.cache_sizes();
    assert!(decoded <= 64 + masked * 2, "decoded cache is bounded by map tiles and the other visible masks");
    eprintln!(
        "minimap presenter: {:.3} ms/frame, {} bytes/frame; {masked} masked slots, {decoded} decoded images",
        elapsed.as_secs_f64() * 1000.0 / FRAMES as f64,
        bytes / FRAMES
    );
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_race_and_interior_map_icons_are_objects_of_the_single_race_hud() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let package = catalog.find("HUD_SingleRace.fng").expect("HUD_SingleRace.fng").clone();
    let mut rt = Runtime::new();
    let id = rt.load(package);
    for style in super::blip_styles::map_icon_styles() {
        for icon in style.icons() {
            assert!(rt.find(id, fe_hash_upper(&icon)).is_some(), "{icon} is an object of the HUD package");
        }
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_pursuit_breaker_icons_are_a_pool_of_the_single_race_hud() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let package = catalog.find("HUD_SingleRace.fng").expect("HUD_SingleRace.fng").clone();
    let mut rt = Runtime::new();
    let id = rt.load(package);
    let found: Vec<usize> =
        (0..128).filter(|i| rt.find(id, fe_hash_upper(&format!("PURSUIT_BREAKER_{i}"))).is_some()).collect();
    eprintln!("PURSUIT_BREAKER_n objects: {found:?}");
    assert_eq!(found, (0..super::blip_styles::PURSUIT_BREAKER_POOL).collect::<Vec<_>>());
}

#[test]
#[ignore = "needs installed gauge packs"]
fn installed_gauge_styles_have_decodable_faces_needles_and_fills() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let assets = UiAssets::load(&dir).unwrap();
    for skin in 0..=10 {
        for hash in (7..=10).map(|rpm| super::skin::tach_face_texture(rpm as f32 * 1000.0, skin)).chain([
            super::skin::needle_texture(skin),
            super::skin::fill_texture(skin),
            fe_hash_upper(&format!("TURBO_LINES_{skin:02}")),
            fe_hash_upper(&format!("TURBO_NEEDLE_{skin:02}")),
        ]) {
            let texture =
                assets.texture(hash).unwrap_or_else(|| panic!("missing gauge texture {hash:08X} for style {skin}"));
            assert!(blackbox_tpk::decode_rgba8(&texture).is_some());
        }
    }
}
