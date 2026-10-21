//! With the install (`NFSMW_GAME_DIR`; the tests return at once without it): the four packages, their halves, their
//! textures, and both HUDs drawn from synthetic states. With `NFSMW_SHOT_DIR` set the frames are also written there as PNGs.

use blackbox_feng::{NodeKind, UiTree};
use blackbox_minimap::Orientation;
use game_install::GameDir;
use glam::Vec3;

use super::pair::Pair;
use super::raster_tests::{Canvas, Textures};
use super::{Kind, Seat, package};
use crate::gui::UiOutput;
use crate::hud::race::{Count, Message, RaceHud};
use crate::hud::state::{HudState, MapPosition};
use crate::settings::HudLayout;
use crate::ui::UiAssets;
use crate::ui::present::{BlackboxPresenter, Screen};

/// The units a HUD may reach over the middle of the canvas (the backing panels are drawn a few units wide of their centres).
const TOLERANCE: f32 = 10.0;

fn install() -> Option<GameDir> {
    let root = std::env::var_os("NFSMW_GAME_DIR")?;
    GameDir::open(std::path::PathBuf::from(root)).ok()
}

/// The vertical extent in canvas units of what a package draws at rest: the corners of its images and the origins of its text.
fn extent(tree: &UiTree, skip: impl Fn(u32) -> bool) -> (f32, f32) {
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for &i in &tree.draw_order {
        let node = &tree.nodes[i];
        if skip(node.guid) {
            continue;
        }
        let corners: &[Vec3] = match node.kind {
            NodeKind::Image { .. } => &[Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, 0.5, 0.0)],
            NodeKind::Text { .. } => &[Vec3::ZERO],
            _ => continue,
        };
        for corner in corners {
            let y = node.world.transform_point3(*corner).y;
            low = low.min(y);
            high = high.max(y);
        }
    }
    (low, high)
}

#[test]
fn the_bun_has_the_four_packages_and_each_is_authored_for_its_own_half_of_the_canvas() {
    let Some(dir) = install() else { return };
    for kind in Kind::BOTH {
        let [first, second] = package::load(&dir, kind).expect("the pair is in the bun");
        assert_eq!(first.name, kind.package(Seat::First));
        assert_eq!(second.name, kind.package(Seat::Second));
        // The speed-trap icons are never shown (and are authored below the canvas); the rule and the first player's
        // minimap, which sit on the middle, belong to both halves.
        let traps: Vec<u32> = first
            .objects
            .iter()
            .filter(|o| {
                let name = o.resource.and_then(|r| first.resources.get(r)).map(|r| r.name.to_ascii_lowercase());
                name.is_some_and(|n| n.starts_with("minimap_icon_speedtrap"))
            })
            .map(|o| o.guid)
            .collect();
        let mut rt = crate::ui::runtime();
        let ids = [rt.load(first), rt.load(second)];
        let shared = |guid: u32| {
            traps.contains(&guid)
                || guid == super::names::DIVIDER_GUID
                || [0xE2991, 0xE2992, 0xE2993, 0xE2994, 0xE2995, 0xE2A47, 0xE2A20, 0xE298F, 0xF6986].contains(&guid)
        };
        let (low, high) = extent(&rt.tree(ids[0]), shared);
        println!("{kind:?} first: y from {low:.1} to {high:.1}");
        assert!(low >= -240.0 && high <= TOLERANCE, "the first player's HUD is the top half: {low}..{high}");
        let (low, high) = extent(&rt.tree(ids[1]), |_| false);
        println!("{kind:?} second: y from {low:.1} to {high:.1}");
        assert!(low >= -TOLERANCE && high <= 240.0, "the second player's HUD is the bottom half: {low}..{high}");
    }
}

#[test]
fn every_texture_of_the_packages_resolves() {
    let Some(dir) = install() else { return };
    let assets = UiAssets::load(&dir).unwrap();
    for kind in Kind::BOTH {
        for package in package::load(&dir, kind).unwrap() {
            // The multi-image placeholder is no texture, and the map's picture is swapped in by the minimap binding, as in
            // the single-player HUD.
            let missing: Vec<&str> = assets
                .missing_resources(&package)
                .into_iter()
                .filter(|name| !["(Multi-Textured Image)", "Track2000_map.tga"].contains(name))
                .collect();
            println!("{}: missing {missing:?}", package.name);
            assert!(missing.is_empty(), "{}: {missing:?}", package.name);
        }
    }
}

/// A moment of a race for one player.
fn state(place: u32, kmh: f32) -> HudState {
    HudState {
        speed: kmh / 3.6,
        rpm: 6200.0,
        max_rpm: 8000.0,
        red_line: 7500.0,
        gear: 4,
        has_nos: true,
        nos: 0.6,
        race: Some(RaceHud { place, racers: 2, lap: Some((2, 3)), clock: 83.4, progress: 0.5, count: None }),
        minimap: Some(MapPosition { position: [1925.0, 1874.0], heading: [1.0, 0.0], orientation: Orientation::North }),
        ..Default::default()
    }
}

/// Runs a pair for a second, draws both trees into an image and returns it.
fn frame(pair: &mut Pair, assets: &UiAssets, states: &[HudState; 2], screen: Screen, layout: HudLayout) -> Canvas {
    for _ in 0..60 {
        pair.step(states, 1.0 / 60.0, assets);
    }
    let mut presenter = BlackboxPresenter::default();
    let mut textures = Textures::default();
    let mut canvas = Canvas::new(screen.width as u32, screen.height as u32, [52, 58, 66]);
    for tree in pair.trees(screen, layout) {
        let mut out = UiOutput::default();
        presenter.present(&tree, assets, screen, &mut out);
        textures.take(&mut out);
        canvas.draw(&out, &textures);
    }
    canvas
}

fn save(name: &str, canvas: &Canvas) {
    let Some(dir) = std::env::var_os("NFSMW_SHOT_DIR") else { return };
    std::fs::create_dir_all(&dir).unwrap();
    canvas.save(&std::path::Path::new(&dir).join(format!("{name}.png")));
}

#[test]
fn both_race_huds_draw_in_their_halves_and_the_second_has_no_map() {
    let Some(dir) = install() else { return };
    let assets = UiAssets::load(&dir).unwrap();
    let mut pair = Pair::load(&dir, &assets, Kind::Race).expect("the race pair");
    let screen = Screen { width: 1280.0, height: 720.0, pixels_per_point: 1.0 };
    let message = Message { serial: 1, text: "CHECKPOINT".into() };
    let mut first = state(1, 150.0);
    first.message = Some(message);
    let states = [first, HudState { shift_light: true, rpm: 7600.0, gear: 3, ..state(2, 120.0) }];
    let canvas = frame(&mut pair, &assets, &states, screen, HudLayout::Pc);
    save("race_16x9", &canvas);
    // Something is drawn over each half's right side (the gauges), and not over the middle of the window (the world).
    let changed =
        |x0: u32, x1: u32, y0: u32, y1: u32| (y0..y1).any(|y| (x0..x1).any(|x| canvas.rgb(x, y) != [52, 58, 66]));
    assert!(changed(900, 1280, 20, 360), "the first player's gauges");
    assert!(changed(900, 1280, 380, 720), "the second player's gauges");
    assert!(!changed(520, 760, 160, 330), "the middle of the first half is the world's");
    assert!(!changed(520, 760, 420, 700), "the middle of the second half is the world's");
}

#[test]
fn the_race_hud_draws_at_other_window_shapes() {
    let Some(dir) = install() else { return };
    let assets = UiAssets::load(&dir).unwrap();
    let mut pair = Pair::load(&dir, &assets, Kind::Race).expect("the race pair");
    let mut first = state(1, 150.0);
    first.race = first.race.map(|r| RaceHud { count: Some(Count::Two), ..r });
    let states = [first, state(2, 120.0)];
    for (name, width, height) in
        [("race_4x3", 1024.0, 768.0), ("race_21x9", 1720.0, 720.0), ("race_square", 720.0, 720.0)]
    {
        let screen = Screen { width, height, pixels_per_point: 1.0 };
        let canvas = frame(&mut pair, &assets, &states, screen, HudLayout::Pc);
        save(name, &canvas);
    }
}

#[test]
fn both_drag_huds_draw() {
    let Some(dir) = install() else { return };
    let assets = UiAssets::load(&dir).unwrap();
    let mut pair = Pair::load(&dir, &assets, Kind::Drag).expect("the drag pair");
    let screen = Screen { width: 1280.0, height: 720.0, pixels_per_point: 1.0 };
    let drag = |s: HudState| HudState { drag: true, minimap: None, ..s };
    let states = [drag(state(1, 150.0)), drag(HudState { shifting: true, ..state(2, 120.0) })];
    let canvas = frame(&mut pair, &assets, &states, screen, HudLayout::Pc);
    save("drag_16x9", &canvas);
    let changed = (0..720).any(|y| (0..1280).any(|x| canvas.rgb(x, y) != [52, 58, 66]));
    assert!(changed, "the drag HUDs draw something");
}
