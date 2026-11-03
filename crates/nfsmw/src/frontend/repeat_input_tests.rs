//! Held settings directions through the installed Options screen and FEng input runtime.

use super::*;
use blackbox_feng::fe_hash_upper;

#[track_caller]
fn selected(h: &Harness, row: usize) {
    let tree = h.screens.trees().pop().expect("options tree");
    let name = fe_hash_upper(&format!("OPTION_NAME_{}", row + 1));
    let node = tree.nodes.iter().find(|n| n.name_hash == name).expect("row name");
    let focus = h.screens.focused_button().expect("selected setting has focus");
    assert_eq!(
        focus.index,
        node.index,
        "selected row {row}; actual focus hash={:08x}, text={:?}, visible={} ; cursor={:?}",
        tree.nodes[focus.index].name_hash,
        tree.nodes[focus.index].text,
        tree.nodes[focus.index].visible,
        tree.nodes.iter().find(|n| n.name_hash == 0x0674_5352).map(|n| n.world.w_axis)
    );
}

fn steps(h: &mut Harness, mask: u32, dt: f32, count: usize) {
    for _ in 0..count {
        let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
        h.commands.extend(h.screens.update(dt, mask, &mut env));
    }
}

fn player() -> Harness {
    let mut h = Harness::open(screen::OPTIONS, Args { category: Category::Player, ..Args::default() })
        .expect("installed Options package");
    h.wait(1.5);
    selected(&h, 0);
    h
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn installed_settings_held_directions_repeat_release_and_reverse() {
    let mut h = player();
    steps(&mut h, pad::DOWN, FRAME, 1);
    selected(&h, 1);
    steps(&mut h, pad::DOWN, FRAME, 15);
    selected(&h, 1);
    steps(&mut h, pad::DOWN, FRAME, 1);
    selected(&h, 2);
    steps(&mut h, pad::DOWN, FRAME, 6);
    selected(&h, 0);
    steps(&mut h, pad::UP, FRAME, 1);
    selected(&h, 2);
    steps(&mut h, pad::UP, FRAME, 15);
    selected(&h, 2);
    steps(&mut h, pad::UP, FRAME, 1);
    selected(&h, 1);
    steps(&mut h, 0, FRAME, 1);
    steps(&mut h, pad::DOWN, FRAME, 1);
    selected(&h, 2);
    steps(&mut h, pad::DOWN, FRAME, 15);
    selected(&h, 2);
    steps(&mut h, pad::DOWN, FRAME, 1);
    selected(&h, 0);
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn installed_settings_held_down_keeps_elapsed_repeat_count_at_common_frame_rates() {
    for hz in [60, 120, 144] {
        let mut h = player();
        steps(&mut h, pad::DOWN, 1.0 / hz as f32, hz);
        // Nine movements at the frontend's 1200 Hz clock: initial, first repeat and seven fast repeats.
        selected(&h, 0);
    }
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn installed_settings_new_screen_resets_repeat_after_a_held_transition() {
    let mut h = player();
    steps(&mut h, pad::DOWN, FRAME, 17);
    selected(&h, 2);
    let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
    h.commands.extend(h.screens.open(
        screen::OPTIONS,
        Args { category: Category::Player, ..Args::default() },
        true,
        &mut env,
    ));
    // The incoming screen gates the still-held input, then grants control on release.
    steps(&mut h, pad::DOWN, FRAME, 1);
    steps(&mut h, 0, FRAME, 1);
    h.wait(1.5);
    selected(&h, 0);
    steps(&mut h, pad::DOWN, FRAME, 1);
    selected(&h, 1);
    steps(&mut h, pad::DOWN, FRAME, 6);
    selected(&h, 1);
    steps(&mut h, pad::DOWN, FRAME, 10);
    selected(&h, 2);
}

fn assert_cadence(h: &mut Harness, hz: usize) {
    let mut times = Vec::new();
    let mut last = h.screens.focused_button().unwrap().index;
    for frame in 0..hz {
        steps(h, pad::DOWN, 1.0 / hz as f32, 1);
        let next = h.screens.focused_button().unwrap().index;
        if next != last {
            times.push((frame + 1) as f32 / hz as f32);
            last = next;
        }
    }
    assert_eq!(times.len(), 9, "same one-second hold at {hz} Hz: {times:?}");
    let tolerance = 2.0 / hz as f32 + 1.0 / 1200.0;
    for (index, pair) in times.windows(2).enumerate() {
        let expected = match index {
            0 => 320.0 / 1200.0,
            _ => 120.0 / 1200.0,
        };
        assert!((pair[1] - pair[0] - expected).abs() <= tolerance, "unsteady interval at {hz} Hz: {times:?}");
    }
    steps(h, 0, 1.0 / hz as f32, 1);
}

#[test]
#[ignore = "requires installed menu packages"]
fn installed_settings_repeat_cadence_is_steady_above_the_ui_tick_rate() {
    for hz in [60, 144, 1200, 2163, 2400, 4800] {
        let mut h = player();
        assert_cadence(&mut h, hz);
        h.press(pad::BACK);
        h.wait(1.0);
        assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
        for _ in 0..3 {
            h.press(pad::RIGHT);
        }
        h.press(pad::ACCEPT);
        h.wait(1.0);
        assert_eq!(h.screens.top(), Some(screen::OPTIONS));
        selected(&h, 0);
        assert_cadence(&mut h, hz);
    }
}
