//! UI-031 is the career race list: verify actual RacePage slot artwork, state, detail synchronization and loaded assets.

use blackbox_feng::runtime::pad;
use blackbox_feng::{NodeKind, fe_hash_upper};
use game_install::GameDir;

use super::*;
use crate::frontend::career_screens::rep_sheets::{Kind, Sheet, icons, playtest::Harness};
use crate::frontend::challenge_series::{Maps, Zoom};
use crate::frontend::logic::Args;

const KINDS: [(RaceType, u32); 6] = [
    (RaceType::Sprint, 0x2521_E5EB),
    (RaceType::Circuit, 0xE963_8D3E),
    (RaceType::Drag, 0xAAAB_31E9),
    (RaceType::Knockout, 0x3A01_5595),
    (RaceType::Tollbooth, 0x1A09_1045),
    (RaceType::SpeedTrap, 0x66C9_A7B6),
];

fn fixture() -> RacePage {
    let rows = (0..13)
        .map(|at| RaceRow {
            event: format!("15.1.{}", at + 1),
            kind: KINDS[at % KINDS.len()].0,
            boss: at > 8,
            status: match at % 3 {
                0 => Status::Open,
                1 => Status::Locked,
                _ => Status::Done,
            },
            cash: 1_000,
            laps: 2,
            length: 1_000.0,
            map: 0,
            crop: Crop::WHOLE,
        })
        .collect();
    RacePage { bin: 15, rows }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn career_playtest_race_list_uses_each_event_type_icon_and_keeps_details_on_the_selected_slot() {
    let mut h = Harness::installed();
    for (_, texture) in KINDS {
        assert!(h.screens.assets().texture(texture).is_some(), "career race artwork {texture:08x} must decode");
    }
    let sheet = Sheet {
        kind: Kind::Race,
        bin: 15,
        page: Box::new(fixture()),
        at: 0,
        icons: icons::Icons::default(),
        maps: Maps::default(),
        zoom: Zoom::at(Crop::WHOLE),
        shown_map: None,
        vertical: false,
        args: Args::default(),
    };
    h.open(crate::frontend::ids::screen::SAFE_HOUSE_RACE_SHEET, Box::new(sheet));
    // Visit every fixture row through real package input, including all open/locked/done variants and overflow.
    let path = [
        (0, 0),
        (pad::RIGHT, 1),
        (pad::RIGHT, 2),
        (pad::DOWN, 5),
        (pad::LEFT, 4),
        (pad::LEFT, 3),
        (pad::DOWN, 6),
        (pad::RIGHT, 7),
        (pad::RIGHT, 8),
        (pad::DOWN, 11),
        (pad::LEFT, 10),
        (pad::LEFT, 9),
        (pad::DOWN, 12),
    ];
    let mut first = 0;
    for (mask, selected) in path {
        if mask != 0 {
            h.press(mask);
        }
        if selected >= first + 9 {
            first = (selected / 3 + 1 - 3) * 3;
        }
        let expected = KINDS[selected % KINDS.len()].1;
        let slot = h.node(fe_hash_upper(&format!("EVENT_ICON_{}", selected - first + 1)));
        assert!(
            slot.visible && slot.z > 0.0 && h.tree().draw_order.contains(&slot.index),
            "selected slot {selected} is drawn"
        );
        assert!(
            matches!(slot.kind, NodeKind::Image { texture, .. } if texture == expected),
            "race {selected}: wrong list icon"
        );
        let detail = h.node(KIND_ICON);
        assert!(
            detail.visible && matches!(detail.kind, NodeKind::Image { texture, .. } if texture == expected),
            "race {selected}: details use same artwork"
        );
        assert_eq!(h.node(super::super::COUNTER_NOW).text, Some((selected + 1).to_string()));
        let kind_name = game_text(text::mode_label(KINDS[selected % KINDS.len()].0));
        assert_eq!(h.node(KIND_NAME).text.as_deref(), Some(kind_name.as_str()), "race {selected}: matching type label");
        for slot in 0..9 {
            let row = first + slot;
            if row >= 13 {
                continue;
            }
            let icon = h.node(fe_hash_upper(&format!("EVENT_ICON_{}", slot + 1)));
            let expected = KINDS[row % KINDS.len()].1;
            assert!(
                matches!(icon.kind, NodeKind::Image { texture, .. } if texture == expected),
                "window row {row}: stale Circuit placeholder"
            );
        }
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn career_playtest_installed_career_races_have_loaded_type_artwork_for_every_bin() {
    let dir =
        GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR"))).unwrap();
    let gameplay = nfsmw_data::gameplay::load(&dir).unwrap();
    let data = crate::career::CareerData::from_parts(gameplay, &[], None, None);
    let blacklist = Blacklist::new(data.bins.clone(), 15).unwrap();
    let assets = crate::ui::UiAssets::load(&dir).unwrap();
    let mut kinds = std::collections::BTreeSet::new();
    let mut count = 0;
    for bin in 1..=15 {
        let rows = race_rows(&blacklist, &data.gameplay.races, None, bin);
        assert!(!rows.is_empty(), "career bin {bin} must exercise race-list entries");
        let page = RacePage { bin, rows };
        for (at, row) in page.rows.iter().enumerate() {
            let texture = page.icon(at).unwrap();
            assert!(
                assets.texture(texture).is_some(),
                "bin {bin} race {}: missing type artwork {texture:08x}",
                row.event
            );
            if let Some((kind, expected)) = KINDS.iter().find(|(kind, _)| *kind == row.kind) {
                assert_eq!(texture, *expected, "bin {bin} race {}", row.event);
                kinds.insert(*kind as u8);
            }
            count += 1;
        }
    }
    assert!(count > 15, "installed race-list coverage must include more than one event per bin");
    assert_eq!(kinds.len(), KINDS.len(), "all six career race icon types are covered");
}
