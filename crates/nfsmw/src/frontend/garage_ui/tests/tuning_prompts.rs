//! Tuning must replace the complete legacy PC hints, including their separate key art.
use super::{Harness, Page, model};
use crate::input::{Bindings, InputDevice};
use blackbox_feng::{fe_hash_upper, runtime::pad};

#[test]
#[ignore = "requires installed original garage packages"]
fn batch10_tuning_navigation_restores_departed_row_colour() {
    let mut model = model();
    model.draft.performance.levels.fill(1);
    let mut h = Harness::open(Page::Tuning, model);
    let before = h.screens.trees().pop().unwrap();
    let hash = |row| fe_hash_upper(&format!("OPTION_NAME_{row}"));
    let neutral = before.nodes.iter().find(|n| n.name_hash == hash(2)).unwrap().world_colour;
    for bits in [pad::DOWN, pad::DOWN, pad::UP, pad::DOWN] {
        h.press(bits);
        let tree = h.screens.trees().pop().unwrap();
        let rows: Vec<_> = tree.nodes.iter().filter(|n| (1..=7).any(|i| n.name_hash == hash(i))).collect();
        assert_eq!(rows.len(), 7);
        let active: Vec<_> = rows
            .iter()
            .filter(|n| n.world_colour[..3] != neutral[..3])
            .map(|n| (n.name_hash, n.world_colour))
            .collect();
        assert_eq!(active.len(), 1, "one active tuning label: {active:?}");
        for row in rows {
            let initial = before.nodes.iter().find(|n| n.name_hash == row.name_hash).unwrap();
            assert_eq!(row.world_colour[3], initial.world_colour[3], "disabled-row opacity is preserved");
        }
    }
}

#[test]
#[ignore = "requires installed original garage packages"]
fn batch10_tuning_footer_replaces_native_pc_hint_subtrees() {
    let mut h = Harness::open(Page::Tuning, model());
    for _ in 0..12 {
        for device in [InputDevice::Xbox, InputDevice::Keyboard] {
            let shown = h.screens.presented(&Bindings::default(), device);
            let tree = &shown.last().unwrap().tree;
            for hash in [0xd9e6_3061, 0x9502_d148, 0x5232_50e9] {
                let root = tree.nodes.iter().find(|n| n.name_hash == hash).expect("native tuning PC hint");
                assert!(!root.visible, "PC hint root {hash:08x} stays hidden");
                assert!(tree.nodes.iter().filter(|n| n.parent == Some(root.index)).all(|n| !n.visible));
            }
            for label in ["Cancel", "Accept", "Help"] {
                assert_eq!(
                    tree.nodes.iter().filter(|n| n.visible && n.text.as_deref() == Some(label)).count(),
                    1,
                    "one {label} hint"
                );
            }
        }
        h.run(0, 1);
    }
}
