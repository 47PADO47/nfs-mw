//! With the install (`NFSMW_GAME_DIR`): the HUD package has every object the binding looks for.

use blackbox_feng::{Runtime, fe_hash_upper};
use game_install::GameDir;

use super::bind::HudBinding;
use super::minimap::MinimapBinding;
use crate::ui::{Catalog, UiAssets};

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
