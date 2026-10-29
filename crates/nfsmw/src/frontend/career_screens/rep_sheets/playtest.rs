//! Installed-package reproductions for the career sheet playtest reports (UI-029/030).

use std::sync::Arc;

use blackbox_feng::runtime::pad;
use blackbox_feng::{NodeKind, UiNode, UiTree, fe_hash_upper};
use game_install::GameDir;

use super::*;
use crate::frontend::logic::Env;
use crate::frontend::screens::Screens;
use crate::settings::{Partial, Settings, test_caps};
use crate::ui::{Catalog, SCREEN_FILES};

pub(in crate::frontend::career_screens) struct Harness {
    pub screens: Screens,
    settings: Settings,
    changed: Partial,
    pub commands: Vec<Command>,
}

impl Harness {
    pub fn installed() -> Self {
        let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
        let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
        let assets = Arc::new(UiAssets::load(&dir).unwrap());
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        Self {
            screens: Screens::new(catalog, assets),
            settings: Settings::from(Partial::default()),
            changed: Partial::default(),
            commands: Vec::new(),
        }
    }

    pub fn open(&mut self, name: &str, logic: Box<dyn ScreenLogic>) {
        let mut env =
            Env { settings: &mut self.settings, changed: &mut self.changed, caps: test_caps::native(), mods: None };
        self.commands = self.screens.open_with(name, logic, true, &mut env);
        self.run(0, 100);
    }

    pub fn run(&mut self, mask: u32, frames: usize) {
        for _ in 0..frames {
            let mut env =
                Env { settings: &mut self.settings, changed: &mut self.changed, caps: test_caps::native(), mods: None };
            self.commands.extend(self.screens.update(1.0 / 60.0, mask, &mut env));
        }
    }

    pub fn press(&mut self, mask: u32) {
        self.run(mask, 2);
        self.run(0, 30);
    }

    pub fn wheel(&mut self, steps: i32) {
        let mut env =
            Env { settings: &mut self.settings, changed: &mut self.changed, caps: test_caps::native(), mods: None };
        self.commands.extend(self.screens.scroll(steps, &mut env));
        self.run(0, 30);
    }

    pub fn tree(&self) -> UiTree {
        self.screens.trees().pop().expect("a visible package")
    }

    pub fn presented(&self, device: crate::input::InputDevice) -> UiTree {
        self.screens.presented_trees(&crate::input::Bindings::default(), device).pop().unwrap()
    }

    pub fn click(&mut self, point: Vec2) {
        let mut env =
            Env { settings: &mut self.settings, changed: &mut self.changed, caps: test_caps::native(), mods: None };
        self.commands.extend(self.screens.click(point, &mut env).expect("sheet owns the click"));
        self.run(0, 30);
    }

    pub fn node(&self, hash: u32) -> UiNode {
        self.tree().nodes.into_iter().find(|n| n.name_hash == hash).unwrap_or_else(|| panic!("{hash:#x}"))
    }
}

struct FixturePage;

impl Page for FixturePage {
    fn len(&self) -> usize {
        13
    }
    fn status(&self, at: usize) -> Status {
        match at {
            1 => Status::Locked,
            2 => Status::Done,
            _ => Status::Open,
        }
    }
    fn icon(&self, at: usize) -> Option<u32> {
        (at < self.len()).then(|| crate::frontend::quick_play::text::mode_icon(nfsmw_data::gameplay::RaceType::Sprint))
    }
    fn name(&self, at: usize) -> String {
        format!("row {at}")
    }
    fn summary(&self, at: usize) -> String {
        format!("details {at}")
    }
    fn heading(&self) -> u32 {
        blackbox_text::label_hash("REP_SHEET_RACE_EVENTS")
    }
    fn fill(&self, cx: &mut Cx, at: usize) {
        cx.text_object(COUNTER_NOW, &(at + 1).to_string());
    }
    fn ask(&self, at: usize) -> Option<Ask> {
        (self.status(at) != Status::Locked).then(|| Ask {
            label: blackbox_text::label_hash("REP_SHEET_RACE_CONFIRM"),
            then: Command::LaunchEvent { event: format!("test-{at}") },
        })
    }
}

fn sheet(kind: Kind) -> Sheet {
    Sheet {
        kind,
        bin: 15,
        page: Box::new(FixturePage),
        at: 0,
        icons: icons::Icons::default(),
        maps: Maps::default(),
        zoom: Zoom::at(Crop::WHOLE),
        shown_map: None,
        vertical: false,
        args: Args::default(),
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn career_playtest_sheet_icons_are_visible_on_each_page() {
    let mut h = Harness::installed();
    for (kind, name) in [
        (Kind::Race, screen::SAFE_HOUSE_RACE_SHEET),
        (Kind::Milestones, screen::SAFE_HOUSE_MILESTONES),
        (Kind::Bounty, screen::SAFE_HOUSE_BOUNTY),
    ] {
        h.open(name, Box::new(sheet(kind)));
        for slot in 1..=9 {
            let icon = h.node(fe_hash_upper(&format!("EVENT_ICON_{slot}")));
            assert!(icon.visible, "{name}: icon {slot} is hidden");
            assert!(matches!(icon.kind, NodeKind::Image { .. }));
            assert!(
                matches!(icon.kind, NodeKind::Image { texture: 0x2521_E5EB, .. }),
                "{name}: stale circuit placeholder"
            );
        }
        assert!(h.node(0x688A_1551).visible, "{name}: scrollbar backing");
        assert!(h.node(0x3FBB_7A4E).visible, "{name}: scrollbar handle");
        let lock = [0x7AD9_3592, 0xE636_228E]
            .into_iter()
            .find_map(|hash| h.tree().nodes.into_iter().find(|n| n.name_hash == hash))
            .unwrap();
        assert!(
            lock.visible && matches!(lock.kind, NodeKind::Image { texture: 0x0018_ED48, .. }),
            "{name}: locked row marker"
        );
        let done = [0x7AD9_3593, 0xE636_228F]
            .into_iter()
            .find_map(|hash| h.tree().nodes.into_iter().find(|n| n.name_hash == hash))
            .unwrap();
        assert!(
            done.visible && matches!(done.kind, NodeKind::Image { texture: 0x028F_EADD, .. }),
            "{name}: completed row marker"
        );
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn career_playtest_sheet_down_moves_one_authored_grid_row_and_scrolls() {
    let mut h = Harness::installed();
    h.open(screen::SAFE_HOUSE_MILESTONES, Box::new(sheet(Kind::Milestones)));
    h.press(pad::DOWN);
    assert_eq!(h.node(COUNTER_NOW).text.as_deref(), Some("4"), "the package has three columns");
    h.press(pad::DOWN);
    h.press(pad::DOWN);
    assert_eq!(h.node(COUNTER_NOW).text.as_deref(), Some("10"));
    assert!(h.node(fe_hash_upper("EVENT_ICON_7")).visible, "selected row stays in the window");
    h.wheel(-1);
    assert_eq!(h.node(COUNTER_NOW).text.as_deref(), Some("13"));
    let last = h.node(fe_hash_upper("EVENT_ICON_7"));
    let unused = h.node(fe_hash_upper("EVENT_ICON_8"));
    assert!(last.visible && !unused.visible, "unused slots leave the final window");
    h.open(screen::SAFE_HOUSE_MILESTONES, Box::new(sheet(Kind::Milestones)));
    assert_eq!(h.node(COUNTER_NOW).text.as_deref(), Some("13"), "reopening keeps selection");
    h.press(pad::ACCEPT);
    assert_eq!(h.screens.top(), Some(screen::DIALOG));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn career_playtest_sheet_click_selects_the_visible_entry_and_locked_rows_do_not_launch() {
    let mut h = Harness::installed();
    h.open(screen::SAFE_HOUSE_RACE_SHEET, Box::new(sheet(Kind::Race)));
    let locked = h.node(fe_hash_upper("EVENT_ICON_2")).position.truncate();
    h.click(locked);
    assert_eq!(h.screens.top(), Some(screen::SAFE_HOUSE_RACE_SHEET));
    assert_eq!(h.node(COUNTER_NOW).text.as_deref(), Some("2"));
    let open = h.node(fe_hash_upper("EVENT_ICON_4")).position.truncate();
    h.click(open);
    assert_eq!(h.screens.top(), Some(screen::DIALOG));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn career_playtest_sheet_footers_have_bound_keyboard_and_controller_icons() {
    use crate::input::InputDevice;
    let mut h = Harness::installed();
    for (kind, name) in [
        (Kind::Race, screen::SAFE_HOUSE_RACE_SHEET),
        (Kind::Milestones, screen::SAFE_HOUSE_MILESTONES),
        (Kind::Bounty, screen::SAFE_HOUSE_BOUNTY),
    ] {
        h.open(name, Box::new(sheet(kind)));
        for device in [InputDevice::Keyboard, InputDevice::Xbox] {
            let tree = h.presented(device);
            if device == InputDevice::Xbox {
                assert!(tree.nodes.iter().any(|n| n.visible && matches!(n.kind, NodeKind::Image { texture, .. } if texture == crate::ui::input_icons::ATLAS)), "{name} {device:?}: missing bound input icons");
            }
            if device == InputDevice::Keyboard {
                assert!(
                    tree.nodes.iter().any(|n| n.visible && n.text.as_deref() == Some("Esc")),
                    "{name}: missing keyboard Back"
                );
            }
            let back = crate::frontend::prompts::HintLabel::Back.text();
            assert_eq!(
                tree.nodes.iter().filter(|n| n.visible && n.text.as_deref() == Some(&back)).count(),
                1,
                "{name} {device:?}: duplicate Back"
            );
        }
    }
}
