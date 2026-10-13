//! Map blips on the widescreen HUD: a blip is drawn on the minimap, so it moves with the minimap's group and stays
//! registered to it at every aspect ratio (`docs/specs/hud-minimap.md`, sections 7 and 8).

use blackbox_feng::package::ObjectKind;
use blackbox_feng::{PackageId, Runtime, UiTree, fe_hash_upper};
use blackbox_minimap::{Calibration, Orientation};
use glam::Vec3;

use super::MapBlip;
use super::bind::HudBinding;
use super::blip_styles::COP;
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

/// The blip's place relative to the minimap's centre, in HUD units.
fn offset_from_centre(tree: &UiTree) -> Vec3 {
    at(tree, COP_ICON, Vec3::ZERO) - at(tree, "TRACK_MAP", Vec3::new(32.0, 32.0, 0.0))
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
    let unshifted = offset_from_centre(&before);
    assert!((unshifted.length() - RIM).abs() < 0.01, "pinned to the rim at 4:3: {unshifted:?}");
    for (width, layout) in [(1920.0, HudLayout::Pc), (2560.0, HudLayout::Pc), (1920.0, HudLayout::Xbox360)] {
        let screen = Screen { width, height: 1080.0, pixels_per_point: 1.0 };
        let mut after = before.clone();
        HudViewport::new(&before, &binding.blip_icon_hashes()).apply(&mut after, screen, layout);
        let (scale, _) = parameters(screen, layout);
        let shifted = offset_from_centre(&after);
        let expected = unshifted * scale;
        assert!(
            (shifted.x - expected.x).abs() < 0.01 && (shifted.y - expected.y).abs() < 0.01,
            "{width} wide, {layout:?}: the blip is {shifted:?}, the minimap says {expected:?}"
        );
        assert!(shifted.truncate().length() <= RIM * scale + 0.01, "inside the disc: {shifted:?}");
    }
}
