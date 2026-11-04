//! With the install (`NFSMW_GAME_DIR`): the milestone board's objects are in the HUD package, and a board with slots
//! writes its progress line and done count.

use game_install::GameDir;

use super::bind::HudBinding;
use super::milestone_board::{BoardSlot, Display, MilestoneBoard};
use super::pursuit::PursuitHud;
use super::state::HudState;
use crate::ui::Catalog;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_milestone_board_shows_its_slots_and_progress_line() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let mut rt = crate::ui::runtime();
    let id = rt.load(catalog.find("HUD_SingleRace.fng").unwrap().clone());
    for name in ["MILESTONE_INFO_GROUP", "MILESTONE_ICON_GROUP", "MILSTONE_ICON_1", "MILESTONE_ICON_BACKING_4"] {
        assert!(rt.find(id, blackbox_feng::fe_hash_upper(name)).is_some(), "{name} is in the HUD package");
    }
    let mut binding = HudBinding::new(&mut rt, id);
    let slot = |value, done| BoardSlot { icon: 0x8780_7869, display: Display::Number, value, goal: 25_000.0, done };
    let state = HudState {
        pursuit: Some(PursuitHud::default()),
        milestone_board: Some(MilestoneBoard { slots: vec![slot(1_500.0, false), slot(30_000.0, true)] }),
        ..HudState::default()
    };
    binding.advance_mirror_fade(0.1);
    binding.apply(&mut rt, &state);
    rt.update(0.1);
    let text = |hash| rt.object(rt.find(id, hash).unwrap()).and_then(|o| o.text.clone());
    assert_eq!(text(0xC3E4_8FBF).as_deref(), Some("1,500/25,000"), "the open milestone's progress");
    assert_eq!(text(0x8946_62C5).as_deref(), Some("1"), "one milestone done");
}
