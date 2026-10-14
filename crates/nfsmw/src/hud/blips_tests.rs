//! Map blips on the widescreen HUD: a blip is drawn on the minimap, so it moves with the minimap's group and stays
//! registered to it at every aspect ratio (`docs/specs/hud-minimap.md`, sections 7 and 8).

use blackbox_feng::package::ObjectKind;
use blackbox_feng::{PackageId, Runtime, UiTree, fe_hash_upper};
use blackbox_minimap::{Calibration, Orientation};
use glam::Vec3;

use super::MapBlip;
use super::bind::HudBinding;
use super::blip_styles::{self, COP, RACE_SPRINT};
use super::minimap::MinimapBinding;
use super::minimap_tests;
use super::state::{HudState, MapPosition};
use super::viewport::{HudViewport, parameters};
use crate::settings::HudLayout;
use crate::ui::present::Screen;

const MAP: Calibration = Calibration { origin: [0.0; 2], width: 8000.0 };
const PLAYER: [f32; 2] = [1250.0, 6750.0];
const COP_ICON: &str = "MMICON_COPCAR_0";
/// The minimap's picture radius at which an element is pinned (`blip::place`'s EDGE, 0.06 of 1024 HUD units).
const RIM: f32 = 0.06 * 1024.0;

fn setup() -> (Runtime, PackageId, HudBinding) {
    let mut package = minimap_tests::package();
    package.objects.push(minimap_tests::object(ObjectKind::Image, 60, COP_ICON, None, [0.0; 2]));
    let mut rt = Runtime::new();
    let id = rt.load(package);
    let minimap = MinimapBinding::new(&rt, id, MAP, "MINI_MAP");
    assert!(minimap.is_some(), "the minimap objects are found");
    let binding = HudBinding::new(&mut rt, id).with_minimap(minimap);
    (rt, id, binding)
}

/// Where a node is on the picture: the minimap's centre is the group's local point (32, 32).
fn at(tree: &UiTree, name: &str, local: Vec3) -> Vec3 {
    let node = tree.nodes.iter().find(|n| n.name_hash == fe_hash_upper(name)).unwrap();
    node.world.transform_point3(local)
}

/// The object's place relative to the minimap's centre, in HUD units.
fn offset_from_centre(tree: &UiTree, icon: &str) -> Vec3 {
    at(tree, icon, Vec3::ZERO) - at(tree, "TRACK_MAP", Vec3::new(32.0, 32.0, 0.0))
}

/// The binding with the map's race and interior styles registered on a package that has their objects, as the HUD
/// plugin does (before the viewport).
fn setup_with_map_icons() -> (Runtime, PackageId, HudBinding) {
    let mut package = minimap_tests::package();
    package.objects.push(minimap_tests::object(ObjectKind::Image, 60, COP_ICON, None, [0.0; 2]));
    let styles = blip_styles::map_icon_styles();
    let mut guid = 1000;
    for style in &styles {
        for icon in style.icons() {
            package.objects.push(minimap_tests::object(ObjectKind::Image, guid, &icon, None, [0.0; 2]));
            guid += 1;
        }
    }
    let mut rt = Runtime::new();
    let id = rt.load(package);
    let minimap = MinimapBinding::new(&rt, id, MAP, "MINI_MAP");
    assert!(minimap.is_some(), "the minimap objects are found");
    let mut binding = HudBinding::new(&mut rt, id).with_minimap(minimap);
    for style in styles {
        binding.add_blip_style(&mut rt, id, style);
    }
    (rt, id, binding)
}

/// Whether the object is shown this frame (not hidden, and its alpha is above zero).
fn drawn(tree: &UiTree, icon: &str) -> bool {
    let hash = fe_hash_upper(icon);
    tree.nodes.iter().any(|n| n.name_hash == hash && n.visible)
}

fn on_the_map(blips: Vec<MapBlip>) -> HudState {
    HudState {
        minimap: Some(MapPosition { position: PLAYER, heading: [0.0, 1.0], orientation: Orientation::North }),
        blips,
        ..HudState::default()
    }
}

fn race_start(at: [f32; 2]) -> MapBlip {
    MapBlip { style: RACE_SPRINT, position: at, heading: [0.0, 1.0], active: false }
}

#[test]
fn a_blip_past_the_rim_stays_registered_to_the_minimap_at_wide_aspects() {
    let (mut rt, id, mut binding) = setup();
    // A cop 900 m east of the player: past the rim, so it is pinned to the edge of the disc.
    let state = HudState {
        minimap: Some(MapPosition { position: PLAYER, heading: [0.0, 1.0], orientation: Orientation::North }),
        blips: vec![MapBlip {
            style: COP,
            position: [PLAYER[0] + 900.0, PLAYER[1]],
            heading: [0.0, 1.0],
            active: false,
        }],
        ..HudState::default()
    };
    binding.apply(&mut rt, &state);
    let before = rt.tree(id);
    let unshifted = offset_from_centre(&before, COP_ICON);
    assert!((unshifted.length() - RIM).abs() < 0.01, "pinned to the rim at 4:3: {unshifted:?}");
    for (width, layout) in [(1920.0, HudLayout::Pc), (2560.0, HudLayout::Pc), (1920.0, HudLayout::Xbox360)] {
        let screen = Screen { width, height: 1080.0, pixels_per_point: 1.0 };
        let mut after = before.clone();
        HudViewport::new(&before, &binding.blip_icon_hashes()).apply(&mut after, screen, layout);
        let (scale, _) = parameters(screen, layout);
        let shifted = offset_from_centre(&after, COP_ICON);
        let expected = unshifted * scale;
        assert!(
            (shifted.x - expected.x).abs() < 0.01 && (shifted.y - expected.y).abs() < 0.01,
            "{width} wide, {layout:?}: the blip is {shifted:?}, the minimap says {expected:?}"
        );
        assert!(shifted.truncate().length() <= RIM * scale + 0.01, "inside the disc: {shifted:?}");
    }
}

#[test]
fn a_race_start_inside_the_disc_is_drawn_on_the_minimap() {
    let (mut rt, id, mut binding) = setup_with_map_icons();
    binding.apply(&mut rt, &on_the_map(vec![race_start([PLAYER[0] + 50.0, PLAYER[1]])]));
    let tree = rt.tree(id);
    assert!(drawn(&tree, "MMICON_SPRINT_0"), "the first race icon is drawn");
    let offset = offset_from_centre(&tree, "MMICON_SPRINT_0");
    assert!(offset.truncate().length() <= RIM + 0.01, "inside the disc: {offset:?}");
}

#[test]
fn a_race_start_far_off_the_disc_takes_no_icon() {
    // The far start is listed first: it is off the disc, so it must not take the icon the near one needs.
    let (mut rt, id, mut binding) = setup_with_map_icons();
    let blips = vec![race_start([PLAYER[0] + 5000.0, PLAYER[1]]), race_start([PLAYER[0] + 50.0, PLAYER[1]])];
    binding.apply(&mut rt, &on_the_map(blips));
    let tree = rt.tree(id);
    assert!(drawn(&tree, "MMICON_SPRINT_0"), "the near start has the first icon");
    assert!(!drawn(&tree, "MMICON_SPRINT_1"), "the far start takes no icon");
}
