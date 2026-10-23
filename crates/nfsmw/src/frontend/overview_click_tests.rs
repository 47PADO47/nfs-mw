//! The mouse on the reputation overview (`career_screens/reputation_overview.rs`): its chevrons, arrows, option and
//! challenge button take clicks. Skipped without `NFSMW_GAME_DIR`.

use std::path::PathBuf;
use std::sync::Arc;

use game_install::GameDir;
use glam::Vec2;
use nfsmw_career::settings::NewCareer;

use super::*;
use crate::career::{Career, CareerData, CareerHandle};

const STEPPER_UP: u32 = 0x47b2_2fca;
const STEPPER_DOWN: u32 = 0x72ad_598c;
const ARROW_LEFT: u32 = 0x3df3_9a82;
const ARROW_RIGHT: u32 = 0xfcd5_b255;
const OPTION_ICON: u32 = 0xb046_69e3;

/// The overview over a career whose current rival is `bin`, or `None` without an install.
fn overview(bin: u8) -> Option<Harness> {
    let root = std::env::var_os("NFSMW_GAME_DIR")?;
    let dir = GameDir::open(PathBuf::from(root)).unwrap();
    let data = CareerData::load(&dir).expect("the career tables load");
    let mut career = Career::new("clicks", Arc::new(data), &NewCareer::default()).unwrap();
    career.blacklist.enter_bin(bin);
    CareerHandle::shared().set(career);
    let mut h = Harness::open(screen::SAFE_HOUSE_REPUTATION_OVERVIEW, Args { safehouse: true, ..Args::default() })?;
    h.wait(2.0);
    Some(h)
}

impl Harness {
    /// The centre of the object with this name hash on the top screen.
    fn centre(&self, hash: u32) -> Vec2 {
        let tree = self.screens.trees().pop().unwrap();
        let node = tree.nodes.iter().find(|n| n.name_hash == hash).unwrap_or_else(|| panic!("no object {hash:#x}"));
        node.world.transform_point3(glam::Vec3::ZERO).truncate()
    }

    /// Clicks the object with this name hash and lets the screen run on a little.
    fn click_on(&mut self, hash: u32) -> bool {
        let at = self.centre(hash);
        let taken = self.click(at);
        self.wait(0.3);
        taken
    }

    /// Whether some text on the top screen starts with `prefix`.
    fn shows(&self, prefix: &str) -> bool {
        let tree = self.screens.trees().pop().unwrap();
        tree.nodes.iter().any(|n| n.visible && n.text.as_deref().is_some_and(|t| t.starts_with(prefix)))
    }
}

#[test]
fn the_chevrons_step_the_rival_and_wrap() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut h) = overview(13) else { return };
    assert!(h.shows("blacklist 13"), "the page opens at the career's rival");
    assert!(h.click_on(STEPPER_UP), "the top chevron takes the click");
    assert!(h.shows("blacklist 14"));
    assert!(h.click_on(STEPPER_DOWN), "the bottom chevron takes the click");
    assert!(h.shows("blacklist 13"));
    assert!(h.click_on(STEPPER_DOWN));
    assert!(h.shows("blacklist 15"), "below the career's rival it wraps to rival 15");
}

#[test]
fn the_arrows_walk_the_sheets_and_the_option_opens_the_bio() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut h) = overview(13) else { return };
    assert!(h.shows("Race Events"));
    assert!(!h.click_on(ARROW_LEFT), "the left arrow is hidden on the first sheet");
    assert!(h.click_on(ARROW_RIGHT));
    assert!(h.shows("Milestone Events"));
    assert!(h.click_on(ARROW_LEFT));
    assert!(h.shows("Race Events"));
    h.click_on(OPTION_ICON);
    assert_eq!(
        h.screens.top(),
        Some(screen::SAFE_HOUSE_REPUTATION_OVERVIEW),
        "a sheet that is not built opens nothing"
    );
    for _ in 0..3 {
        h.click_on(ARROW_RIGHT);
    }
    assert!(h.shows("Bio"));
    assert!(h.click_on(OPTION_ICON), "the option takes the click");
    h.wait(0.5);
    let top = h.screens.top().unwrap_or_default();
    assert!(top.eq_ignore_ascii_case(screen::SAFE_HOUSE_RIVAL_BIO), "the bio opened, the top screen is {top}");
}

#[test]
fn the_pad_steps_picks_opens_and_leaves() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut h) = overview(13) else { return };
    h.press(pad::UP);
    assert!(h.shows("blacklist 14"), "up steps to the next rival");
    h.press(pad::DOWN);
    assert!(h.shows("blacklist 13"));
    h.press(pad::RIGHT);
    assert!(h.shows("Milestone Events"), "right picks the next sheet");
    h.press(pad::LEFT);
    assert!(h.shows("Race Events"));
    for _ in 0..3 {
        h.press(pad::RIGHT);
    }
    h.press(pad::ACCEPT);
    h.wait(0.5);
    let top = h.screens.top().unwrap_or_default();
    assert!(top.eq_ignore_ascii_case(screen::SAFE_HOUSE_RIVAL_BIO), "accept opens the bio, the top screen is {top}");
    let mut h = overview(13).unwrap();
    h.press(pad::BACK);
    h.wait(0.5);
    let top = h.screens.top().unwrap_or_default();
    assert!(top.eq_ignore_ascii_case(screen::MAIN_MENU_SUB), "back leaves for the safe house menu, the top is {top}");
}
