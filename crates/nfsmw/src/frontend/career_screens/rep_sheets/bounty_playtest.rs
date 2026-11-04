use super::*;
use crate::frontend::career_screens::rep_sheets::{Kind, Sheet, icons, playtest::Harness};
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
#[ignore = "requires installed bounty package and regional textures; set NFSMW_GAME_DIR"]
fn career_batch13_bounty_grid_and_detail_share_the_selected_regional_crest() {
    let page = BountyPage { rows: (1..=12).map(|number| BountyRow { number, at: [0.0; 2] }).collect() };
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
