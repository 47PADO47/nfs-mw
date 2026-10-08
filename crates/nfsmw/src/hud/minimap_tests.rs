//! The minimap binding on a synthetic package: the objects the game moves, with the numbers of
//! `docs/specs/hud-minimap.md`.

use blackbox_feng::package::{MultiDef, ObjectData, ObjectDef, ObjectKind, Package};
use blackbox_feng::{NodeKind, PackageId, Runtime, fe_hash_upper};
use blackbox_minimap::{Calibration, Orientation, tile_name};
use glam::{EulerRot, Vec3};

use super::bind::HudBinding;
use super::minimap::MinimapBinding;
use super::state::{HudState, MapPosition};

/// A tile is 1000 m wide: the picture's bottom left corner is the world origin.
const MAP: Calibration = Calibration { origin: [0.0, 0.0], width: 8000.0 };
const GROUP_AT: [f32; 2] = [-221.0, 139.0];
/// The authored mask rectangles of the four pieces, as in the install's package.
const WINDOWS: [[f32; 4]; 4] =
    [[-0.5, -0.5, 0.5, 0.5], [0.5, -0.5, 1.5, 0.5], [-0.5, 0.5, 0.5, 1.5], [0.5, 0.5, 1.5, 1.5]];

fn object(kind: ObjectKind, guid: u32, name: &str, parent: Option<u32>, at: [f32; 2]) -> ObjectDef {
    let mut words = vec![0u32; if kind == ObjectKind::MultiImage { 36 } else { 17 }];
    for (i, v) in [255i32, 255, 255, 255].iter().enumerate() {
        words[i] = *v as u32;
    }
    words[7] = at[0].to_bits();
    words[8] = at[1].to_bits();
    words[10 + 3] = 1.0f32.to_bits();
    ObjectDef {
        kind,
        guid,
        name_hash: fe_hash_upper(name),
        flags: 0,
        resource: None,
        parent,
        data: ObjectData { words },
        string: None,
        multi: (kind == ObjectKind::MultiImage).then(MultiDef::default),
        scripts: Vec::new(),
        responses: Vec::new(),
    }
}

fn package() -> Package {
    let mut objects = vec![
        object(ObjectKind::Group, 1, "SpeedometerGroup", None, [0.0; 2]),
        object(ObjectKind::Group, 5, "GaugeCluster", None, [0.0; 2]),
        object(ObjectKind::Group, 20, "TRACK_MAP", None, GROUP_AT),
        object(ObjectKind::Group, 30, "Ring", None, [0.0; 2]),
        object(ObjectKind::Group, 31, "TRACKMAPTARGETRING", Some(30), [5.0, 132.0]),
        object(ObjectKind::Image, 40, "PLAYERCARINDICATOR", None, [-222.0, 139.0]),
        object(ObjectKind::Group, 50, "RadarGroup", None, [0.0; 2]),
    ];
    for (i, name) in ["TRACK_MAP1", "TRACK_MAP2", "TRACK_MAP3", "TRACK_MAP4"].iter().enumerate() {
        let mut piece = object(ObjectKind::MultiImage, 21 + i as u32, name, Some(20), [0.0; 2]);
        let w = WINDOWS[i];
        for (word, v) in [(21, w[0]), (22, w[1]), (27, w[2]), (28, w[3])] {
            piece.data.words[word] = v.to_bits();
        }
        objects.push(piece);
    }
    Package {
        name: "HUD.fng".into(),
        file_name: String::new(),
        version: 0x20000,
        resources: Vec::new(),
        objects,
        button_count: 0,
        responses: Vec::new(),
        targets: Vec::new(),
    }
}

fn setup() -> (Runtime, PackageId, HudBinding) {
    let mut rt = Runtime::new();
    let id = rt.load(package());
    let minimap = MinimapBinding::new(&rt, id, MAP, "MINI_MAP");
    assert!(minimap.is_some(), "the objects are found");
    let binding = HudBinding::new(&mut rt, id).with_minimap(minimap);
    (rt, id, binding)
}

fn state(at: [f32; 2], heading: [f32; 2], orientation: Orientation) -> HudState {
    HudState { minimap: Some(MapPosition { position: at, heading, orientation }), ..HudState::default() }
}

fn node(rt: &Runtime, id: PackageId, name: &str) -> blackbox_feng::UiNode {
    rt.tree(id).nodes.into_iter().find(|n| n.name_hash == fe_hash_upper(name)).unwrap()
}

fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-3, "{a} vs {b}");
}

fn angle(n: &blackbox_feng::UiNode) -> f32 {
    n.local_rotation.to_euler(EulerRot::ZYX).0.to_degrees()
}

#[test]
fn the_minimap_shows_only_with_a_map_position() {
    let (mut rt, id, mut b) = setup();
    for name in ["TRACK_MAP", "TRACKMAPTARGETRING", "PLAYERCARINDICATOR", "TRACK_MAP3"] {
        assert!(!node(&rt, id, name).visible, "{name} hidden without a position");
    }
    b.apply(&mut rt, &state([1250.0, 6750.0], [0.0, 1.0], Orientation::North));
    for name in ["TRACK_MAP", "TRACKMAPTARGETRING", "PLAYERCARINDICATOR", "TRACK_MAP1", "TRACK_MAP4", "Ring"] {
        assert!(node(&rt, id, name).visible, "{name} shown");
    }
    assert!(!node(&rt, id, "RadarGroup").visible, "the rest of the package stays hidden");
    b.apply(&mut rt, &HudState::default());
    assert!(!node(&rt, id, "TRACK_MAP").visible, "and goes away again");
}

#[test]
fn the_pieces_get_the_tiles_around_the_player() {
    let (mut rt, id, mut b) = setup();
    b.apply(&mut rt, &state([1250.0, 6750.0], [0.0, 1.0], Orientation::North));
    for (i, tile) in [0, 1, 8, 9].into_iter().enumerate() {
        let n = node(&rt, id, &format!("TRACK_MAP{}", i + 1));
        let NodeKind::Image { texture, .. } = n.kind else { panic!("not an image") };
        assert_eq!(texture, fe_hash_upper(&tile_name("MINI_MAP", tile)), "piece {i}");
    }
    // Another quarter of the tile: other columns.
    b.apply(&mut rt, &state([1750.0, 6250.0], [0.0, 1.0], Orientation::North));
    let NodeKind::Image { texture, .. } = node(&rt, id, "TRACK_MAP1").kind else { panic!("not an image") };
    assert_eq!(texture, fe_hash_upper("MINI_MAP_CHOP9"));
}

#[test]
fn the_group_scrolls_and_the_mask_window_follows() {
    let (mut rt, id, mut b) = setup();
    // A quarter of a tile right of and below the block's centre: 32 units.
    b.apply(&mut rt, &state([1250.0, 6750.0], [0.0, 1.0], Orientation::North));
    let group = node(&rt, id, "TRACK_MAP");
    close(group.local_position.x, GROUP_AT[0] - 32.0);
    close(group.local_position.y, GROUP_AT[1] - 32.0);
    close(group.local_pivot.x, 32.0);
    close(group.local_pivot.y, 32.0);
    for i in 0..4 {
        let NodeKind::Image { mask_uv, .. } = node(&rt, id, &format!("TRACK_MAP{}", i + 1)).kind else {
            panic!("not an image")
        };
        let w = WINDOWS[i];
        for (got, want) in mask_uv.iter().zip([w[0] - 0.25, w[1] - 0.25, w[2] - 0.25, w[3] - 0.25]) {
            close(*got, want);
        }
    }
}

#[test]
fn north_up_turns_the_arrow_and_heading_up_turns_the_group() {
    let (mut rt, id, mut b) = setup();
    // Heading east: bearing 90 degrees, clockwise on the screen.
    b.apply(&mut rt, &state([1250.0, 6750.0], [1.0, 0.0], Orientation::North));
    close(angle(&node(&rt, id, "PLAYERCARINDICATOR")), 90.0);
    close(angle(&node(&rt, id, "TRACK_MAP")), 0.0);
    b.apply(&mut rt, &state([1250.0, 6750.0], [1.0, 0.0], Orientation::Heading));
    close(angle(&node(&rt, id, "PLAYERCARINDICATOR")), 0.0);
    close(angle(&node(&rt, id, "TRACK_MAP")), -90.0);
}

#[test]
fn the_player_stays_at_the_centre_of_the_minimap_whatever_the_turn() {
    // The point of the group at the player's offset lands where the package puts the group.
    let (mut rt, id, mut b) = setup();
    for heading in [[0.0, 1.0], [1.0, 0.0], [-0.6, -0.8]] {
        b.apply(&mut rt, &state([1250.0, 6750.0], heading, Orientation::Heading));
        let group = node(&rt, id, "TRACK_MAP");
        let local = Vec3::new(32.0, 32.0, 0.0);
        let on_screen = group.world.transform_point3(local);
        close(on_screen.x, GROUP_AT[0]);
        close(on_screen.y, GROUP_AT[1]);
    }
}
