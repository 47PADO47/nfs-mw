//! Installed icon rows must shrink during exit, including the embedded Controls icon.

use blackbox_feng::{NodeKind, fe_hash_upper};
use glam::Vec2;

use super::*;

#[derive(Clone, Copy, Debug)]
struct Pose {
    size: Vec2,
    center: Vec2,
    alpha: u8,
}

fn row(h: &Harness, count: usize) -> Vec<Pose> {
    let tree = h.screens.trees().pop().unwrap();
    (3..3 + count)
        .map(|slot| {
            let hash = fe_hash_upper(&format!("OPTION_{slot}"));
            let n = tree.nodes.iter().find(|n| n.name_hash == hash).unwrap();
            Pose { size: n.size.abs().truncate(), center: n.local_position.truncate(), alpha: n.world_colour[3] }
        })
        .collect()
}

fn assert_exit(h: &mut Harness, count: usize, button: u32) {
    h.wait(0.5);
    let full = row(h, count);
    assert!(full.iter().any(|p| p.size.x > 60.0));
    let mut previous = full.clone();
    let mut shrunk = false;
    // Observe the native leave script before EXIT_COMPLETE can unload the screen.
    for frame in 0..20 {
        let mask = match frame {
            0 => button,
            _ => 0,
        };
        h.run(mask, 1);
        let current = row(h, count);
        for ((p, before), idle) in current.iter().zip(&previous).zip(&full) {
            assert!(p.size.x <= before.size.x + 0.001, "icon grew during exit: {before:?} -> {p:?}");
            assert!(p.alpha <= before.alpha, "icon became opaque during exit");
            assert!((p.center - idle.center).length() < 0.001, "exit must scale about the icon centre");
            if idle.size.x > 1.0 {
                shrunk |= p.size.x > 0.01 && p.size.x < idle.size.x - 0.01;
            }
        }
        if frame >= 12 {
            assert!(current.iter().all(|p| p.size.x < 0.001 && p.alpha == 0), "outgoing row stayed visible");
        }
        previous = current;
    }
    assert!(shrunk, "the row must animate through intermediate sizes");
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn all_icon_menus_shrink_on_native_accept_back_and_start() {
    for (name, pause, options, count, button, steps) in [
        (screen::MAIN_MENU, false, false, 5, pad::ACCEPT, 4),
        (screen::MAIN_MENU_SUB, false, true, 4, pad::ACCEPT, 2),
        (screen::MAIN_MENU_SUB, false, true, 4, pad::BACK, 0),
        (screen::PAUSE_MENU, true, false, 3, pad::ACCEPT, 1),
        (screen::PAUSE_MENU, true, false, 3, pad::BACK, 0),
        (screen::PAUSE_MENU, true, false, 3, pad::START, 0),
        (screen::PAUSE_MENU, true, true, 4, pad::ACCEPT, 3),
        (screen::PAUSE_MENU, true, true, 4, pad::BACK, 0),
        (screen::PAUSE_MENU, true, true, 4, pad::START, 0),
    ] {
        let mut h = Harness::open(name, Args { pause, options, ..Args::default() }).unwrap();
        h.wait(1.0);
        for _ in 0..steps {
            h.press(pad::RIGHT);
        }
        assert_exit(&mut h, count, button);
    }
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn the_controller_icon_grows_on_return_and_shares_the_row_exit_factor() {
    let mut h = Harness::open(screen::PAUSE_MENU, Args { pause: true, options: true, ..Args::default() }).unwrap();
    assert!(row(&h, 4).iter().all(|p| p.size.x == 0.0 && p.alpha == 0));
    h.wait(1.0);
    for _ in 0..3 {
        h.press(pad::RIGHT);
    }
    h.wait(0.5);
    let tree = h.screens.trees().pop().unwrap();
    let controller = tree.nodes.iter().find(|n| n.name_hash == fe_hash_upper("OPTION_6")).unwrap();
    assert!(matches!(controller.kind, NodeKind::Image { texture: crate::ui::input_icons::ATLAS, .. }));
    let full = row(&h, 4);
    h.run(pad::ACCEPT, 1);
    h.run(0, 3);
    let current = row(&h, 4);
    let factor = current[3].size.x / full[3].size.x;
    assert!(factor > 0.0 && factor < 1.0);
    for (p, idle) in current.iter().zip(&full).filter(|(_, idle)| idle.size.x > 1.0) {
        assert!((p.size.x / idle.size.x - factor).abs() < 0.001);
    }
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::PAUSE_OPTIONS));
    h.wait(1.0);
    h.run(pad::BACK, 1);
    for _ in 0..90 {
        h.run(0, 1);
        if h.screens.top() == Some(screen::PAUSE_MENU) {
            break;
        }
    }
    assert_eq!(h.screens.top(), Some(screen::PAUSE_MENU));
    assert!(row(&h, 4).iter().all(|p| p.size.x == 0.0 && p.alpha == 0), "return must start at zero");
    h.run(0, 3);
    let small = row(&h, 4)[3];
    assert!(small.size.x > 0.0 && small.size.x < 60.0);
    h.wait(1.0);
    assert!(row(&h, 4)[3].size.x > 60.0, "remembered Controls selection must finish growing");
}
