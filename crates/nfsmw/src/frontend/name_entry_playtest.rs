use super::*;
use crate::frontend::name_entry::{NameEntry, NameFor, TextKey};

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn batch12_text_edits_obey_pending_logic_without_releasing_navigation() {
    use crate::frontend::logic::{Cx, ScreenLogic};
    use std::sync::atomic::{AtomicBool, Ordering};

    struct TextProbe(Arc<AtomicBool>);
    impl ScreenLogic for TextProbe {
        fn accepts_input(&self) -> bool {
            self.0.load(Ordering::Relaxed)
        }
        fn wants_text(&self) -> bool {
            true
        }
        fn message(&mut self, _: &mut Cx, _: u32) {}
        fn typed(&mut self, cx: &mut Cx, _: TextKey) {
            cx.send(Command::QuitGame);
        }
        fn pad_pressed(&mut self, cx: &mut Cx, bits: u32) {
            if bits & (pad::ACCEPT | pad::BACK) != 0 {
                cx.send(Command::Resume);
            }
        }
    }
    let mut h = Harness::open(screen::MAIN_MENU, Args::default()).expect("installed menus");
    let ready = Arc::new(AtomicBool::new(false));
    {
        let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
        h.screens.open_with(screen::DIALOG, Box::new(TextProbe(ready.clone())), false, &mut env);
        for key in [TextKey::Char('A'), TextKey::Backspace] {
            assert!(h.screens.type_key(key, &mut env).is_empty(), "pending text logic rejects edits");
        }
        ready.store(true, Ordering::Relaxed);
        for key in [TextKey::Char('A'), TextKey::Backspace] {
            assert_eq!(h.screens.type_key(key, &mut env), vec![Command::QuitGame]);
        }
    }
    h.run(pad::ACCEPT | pad::BACK, 3);
    assert!(!h.said(&Command::Resume), "editing text cannot release the opening navigation action");
    h.run(0, 1);
    h.run(pad::BACK, 1);
    assert_eq!(h.commands.iter().filter(|command| **command == Command::Resume).count(), 1);
}

fn visible_prompt(h: &Harness, phase: &str) {
    let native = h.screens.trees();
    let native = native.last().expect("native name dialog");
    let expected = native.nodes.iter().find(|n| n.name_hash == 0x1e26_40fa).expect(phase);
    assert!(
        expected.visible && expected.world_colour[3] > 0 && native.draw_order.contains(&expected.index),
        "native prompt disappeared during {phase}: text={:?}, visible={}, alpha={}",
        expected.text,
        expected.visible,
        expected.world_colour[3]
    );
    for device in [crate::input::InputDevice::Xbox, crate::input::InputDevice::Keyboard] {
        let trees = h.screens.presented_trees(&crate::input::Bindings::default(), device);
        let tree = trees.last().expect("name dialog");
        assert!(
            tree.nodes.iter().any(|n| n.name_hash == 0x1e26_40fa
                && n.visible
                && n.world_colour[3] > 0
                && tree.draw_order.contains(&n.index)
                && n.text == expected.text
                && n.text.as_ref().is_some_and(|text| text.ends_with('_'))),
            "the {device:?} name prompt must remain drawn and match native text during {phase}"
        );
    }
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn installed_playtest_empty_name_keeps_visible_input_and_allows_cancel_or_valid_submit() {
    for purpose in [NameFor::Create, NameFor::NewCareer] {
        let mut h = Harness::open(screen::MAIN_MENU, Args::default()).expect("installed menus");
        h.wait(1.0);
        let open = |h: &mut Harness| {
            let mut env =
                Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
            h.screens.open_with(screen::DIALOG, Box::new(NameEntry::new(purpose)), false, &mut env);
            for _ in 0..64 {
                h.screens.type_key(TextKey::Backspace, &mut env);
            }
            let empty = format!("{} _", crate::ui::Text::Game(0x2272_9437).show());
            let trees = h.screens.trees();
            let prompt = trees.last().unwrap().nodes.iter().find(|n| n.name_hash == 0x1e26_40fa).unwrap();
            assert_eq!(
                prompt.text.as_deref(),
                Some(empty.as_str()),
                "text edits must clear the buffer before the opening navigation action is released"
            );
            h.commands.clear();
        };
        open(&mut h);
        h.wait(1.5);
        visible_prompt(&h, "before the first empty submission");
        crate::audio::menu::take();
        for frame in 0..90 {
            h.run(pad::ACCEPT, 1);
            assert!(h.commands.is_empty(), "empty Accept emitted a command at frame {frame}");
            visible_prompt(&h, &format!("empty Accept frame {frame}"));
        }
        h.run(0, 6);
        assert_eq!(h.screens.top(), Some(screen::DIALOG));
        assert!(h.commands.is_empty(), "empty input must not create a profile or career");
        visible_prompt(&h, "after empty Accept release");
        assert!(crate::audio::menu::take().is_empty(), "rejected input must not play acceptance");
        let point = {
            let focused = h.screens.focused_button().expect("native Continue button");
            h.screens.trees().last().unwrap().nodes[focused.index].world.w_axis.truncate().truncate()
        };
        assert!(h.screens.focus_button_at(point), "native pointer press targets Continue");
        h.press(pad::ACCEPT);
        h.wait(1.0);
        visible_prompt(&h, "after pointer-focused empty submission");
        assert!(h.commands.is_empty());
        assert!(crate::audio::menu::take().is_empty());
        h.press(pad::BACK);
        h.wait(1.0);
        assert_eq!(h.screens.top(), Some(screen::MAIN_MENU));
        assert_eq!(crate::audio::menu::take(), vec![5], "host-owned Back keeps its existing sound");
        open(&mut h);
        h.wait(1.5);
        {
            let mut env =
                Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
            for c in "   ".chars() {
                h.screens.type_key(TextKey::Char(c), &mut env);
            }
        }
        h.press(pad::ACCEPT);
        h.wait(1.5);
        visible_prompt(&h, "after unsupported whitespace submission");
        assert!(h.commands.is_empty(), "whitespace cannot create an alias");
        {
            let mut env =
                Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
            for c in "UI7NAME".chars() {
                h.screens.type_key(TextKey::Char(c), &mut env);
            }
        }
        assert!(crate::audio::menu::take().iter().all(|id| *id == 10), "typing keeps its own key sound");
        h.run(pad::ACCEPT, 90);
        h.run(0, 6);
        assert_eq!(h.screens.top(), Some(screen::MAIN_MENU));
        let expected = match purpose {
            NameFor::Create => Command::AliasCreate("UI7NAME".into()),
            NameFor::NewCareer => Command::CareerNew("UI7NAME".into()),
        };
        assert_eq!(h.commands, vec![expected], "one valid submission; no empty or repeated command");
        assert_eq!(crate::audio::menu::take(), vec![4], "valid submission accepts exactly once");
    }
}
