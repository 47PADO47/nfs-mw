use super::*;
use crate::frontend::career_screens::rep_sheets::{Kind, MAP, PIN, Sheet, icons, playtest::Harness};
use crate::frontend::challenge_series::{Crop, Maps, Zoom};
use crate::frontend::ids::screen;
use crate::frontend::logic::Args;
use blackbox_feng::{NodeKind, fe_hash_upper, runtime::pad};

#[test]
fn career_batch13_bounty_spawn_numbers_use_their_regional_crests() {
    assert_eq!(place_icon(0), None);
    assert_eq!(place_icon(13), None);
    for number in 1..=12 {
        let expected = [0x8a21_b882, 0x895e_c0ae, 0x9129_e7fb][(number as usize - 1) / 4];
        assert_eq!(place_icon(number), Some(expected));
    }
}

#[test]
fn the_bounty_map_is_centred_on_the_selected_place() {
    let jump = crate::career::JumpPoint::from_marker(glam::Vec3::new(-250.0, 0.0, 500.0), Some(180.0));
    let rows = vec![BountyRow { number: 1, at: [500.0, 250.0], jump }];
    let page = BountyPage { rows, city: Some(Calibration { origin: [0.0, 0.0], width: 1000.0 }) };
    let shown = page.map(0).map(|(texture, crop)| (texture, crop.centre));
    assert_eq!(shown, Some((fe_hash_upper("TRACK_MAP"), [0.5, 0.75])));
    assert!(page.pin(0).is_some_and(|bearing| (bearing - 270.0).abs() < 1e-3));
    assert_eq!((page.map(1), page.pin(1)), (None, None));
}

#[test]
#[ignore = "requires the installed bounty package and TrackMaps.bin; set NFSMW_GAME_DIR"]
fn career_bounty_map_shows_the_city_about_the_place_under_its_pin() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = game_install::GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let city = nfsmw_data::minimap::open_city_calibration(&dir).expect("the open city's calibration");
    // Riverfront Stadium (place 11): map (939.84, -595.59), `Rotation` -102.70.
    let jump = crate::career::JumpPoint::from_marker(glam::Vec3::new(595.59, 0.0, 939.84), Some(-102.70));
    let page = BountyPage { rows: vec![BountyRow { number: 11, at: [939.84, -595.59], jump }], city: Some(city) };
    let (_, crop) = page.map(0).expect("a map");
    let bearing = page.pin(0).expect("a pin");
    let mut h = Harness::installed();
    h.open(
        screen::SAFE_HOUSE_BOUNTY,
        Box::new(Sheet {
            kind: Kind::Bounty,
            bin: 8,
            page: Box::new(page),
            at: 0,
            icons: icons::Icons::default(),
            maps: Maps::default(),
            zoom: Zoom::at(Crop::WHOLE),
            shown_map: None,
            vertical: false,
            args: Args::default(),
        }),
    );
    let map = h.node(MAP);
    assert!(map.visible, "the map picture is shown");
    let NodeKind::Image { texture, uv, .. } = map.kind else { panic!("the map is not an image") };
    assert_eq!(texture, pin::texture());
    assert!(uv.iter().zip(crop.uv()).all(|(got, want)| (got - want).abs() < 1e-4), "{uv:?} against {crop:?}");
    let marker = h.node(PIN);
    assert!(marker.visible, "the pin is over the map");
    let turned = glam::Quat::from_rotation_z(bearing.to_radians());
    assert!(marker.local_rotation.angle_between(turned) < 1e-3, "{:?}", marker.local_rotation);
}

#[test]
#[ignore = "requires installed bounty package and regional textures; set NFSMW_GAME_DIR"]
fn career_batch13_bounty_grid_and_detail_share_the_selected_regional_crest() {
    let page = BountyPage {
        rows: (1..=12)
            .map(|number| BountyRow {
                number,
                at: [0.0; 2],
                jump: crate::career::JumpPoint::from_marker(glam::Vec3::ZERO, None),
            })
            .collect(),
        city: None,
    };
    let mut h = Harness::installed();
    for name in ["BOUNTY_ICON_COLLEGE", "BOUNTY_ICON_COASTAL", "BOUNTY_ICON_ROCKPORT"] {
        let texture = h.screens.assets().texture(fe_hash_upper(name)).expect("installed regional crest");
        let rgba = blackbox_tpk::decode_rgba8(&texture).expect("decode the installed crest without resampling");
        assert!(texture.width >= 64 && texture.width == texture.height, "stock or higher-resolution square crest");
        assert_eq!(rgba.len(), texture.width as usize * texture.height as usize * 4);
        eprintln!("{name}: installed {}x{}", texture.width, texture.height);
    }
    h.open(
        screen::SAFE_HOUSE_BOUNTY,
        Box::new(Sheet {
            kind: Kind::Bounty,
            bin: 8,
            page: Box::new(page),
            at: 0,
            icons: icons::Icons::default(),
            maps: Maps::default(),
            zoom: Zoom::at(Crop::WHOLE),
            shown_map: None,
            vertical: false,
            args: Args::default(),
        }),
    );
    for slot in 1..=9 {
        let node = h.node(fe_hash_upper(&format!("EVENT_ICON_{slot}")));
        assert!(node.visible);
        assert!(matches!(node.kind, NodeKind::Image { texture, .. } if Some(texture) == place_icon(slot)));
    }
    for (mask, number) in [(0, 1), (pad::DOWN, 4), (pad::RIGHT, 5), (pad::DOWN, 8), (pad::DOWN, 11), (pad::RIGHT, 12)] {
        h.press(mask);
        let detail = h.node(0xf97e_c5d5);
        assert!(matches!(detail.kind, NodeKind::Image { texture, .. } if Some(texture) == place_icon(number)));
        let tree = h.tree();
        assert!(tree.nodes.iter().any(|n| n.text.as_deref() == Some(game_text(place_key(number, "")).as_str())));
    }
}
