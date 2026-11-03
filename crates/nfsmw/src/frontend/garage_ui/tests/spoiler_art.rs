//! Installed spoiler strip binding at a non-initial choice.
use super::*;
use blackbox_feng::{NodeKind, fe_hash_upper};
use nfsmw_data::car::customization::Choice;

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_spoiler_strip_binds_visible_icons_after_scrolling() {
    let mut model = model();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    catalog.choices[Part::Spoiler as usize] = (0..14)
        .map(|i| Choice { hash: 4000 + i, label: format!("Spoiler {i}"), language_hash: None, carbon: false })
        .collect();
    let mut h = Harness::open(Page::Spoilers { group: 0, backroom: false }, model);
    for _ in 0..3 {
        h.press(pad::RIGHT);
    }
    h.run(0, 60);
    let tree = h.screens.trees().pop().unwrap();
    let mut visible = 0;
    for index in 1..=80 {
        let hash = fe_hash_upper(&format!("OPTION_{index}"));
        let Some(node) = tree.nodes.iter().find(|n| n.name_hash == hash) else { break };
        if !node.visible || node.world_colour[3] == 0 || !tree.draw_order.contains(&node.index) {
            continue;
        }
        let NodeKind::Image { texture, .. } = node.kind else { panic!("strip slot must be an image") };
        if texture == crate::frontend::ids::END_OF_SCROLLER {
            continue;
        }
        assert_eq!(texture, 0xbb03_4ea6, "ordinary spoiler category art");
        assert!(h.screens.assets().texture(texture).is_some(), "bound spoiler art must be loaded");
        visible += 1;
    }
    assert!(visible >= 3, "several real choices should be visible around the fourth selection");
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_spoiler_filter_rebuild_keeps_authored_strip_geometry() {
    let mut model = model();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    catalog.choices[Part::Spoiler as usize] = (0..16)
        .map(|i| Choice { hash: 5000 + i, label: format!("Spoiler {i}"), language_hash: None, carbon: false })
        .collect();
    for i in 0..16 {
        catalog.part_groups.insert(5000 + i, (i / 8) as u8);
    }
    let mut h = Harness::open(Page::Spoilers { group: 0, backroom: false }, model);
    for selected in [0, 3] {
        if selected == 3 {
            for _ in 0..3 {
                h.press(pad::RIGHT);
            }
        }
        h.run(0, 90);
        let measure = |h: &Harness| {
            let tree = h.screens.trees().pop().unwrap();
            (1..=7)
                .map(|i| {
                    let hash = fe_hash_upper(&format!("OPTION_{i}"));
                    let n = tree.nodes.iter().find(|n| n.name_hash == hash).unwrap();
                    (n.local_position, n.size)
                })
                .collect::<Vec<_>>()
        };
        let before = measure(&h);
        for _ in 0..3 {
            h.click_named(0x7935_4351);
            h.click_named(0x6b36_4f8b);
            h.run(0, 90);
            let after = measure(&h);
            for (slot, (a, b)) in before.iter().zip(&after).enumerate() {
                assert!(
                    (a.0 - b.0).length() < 0.01 && (a.1 - b.1).length() < 0.01,
                    "selection {selected}, slot {slot}: fresh {a:?} rebuilt {b:?}"
                );
            }
        }
    }
}
