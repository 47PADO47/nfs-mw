//! Gauge and status assertions on the actual presented tree, including ancestor fades.

use super::{Harness, Page, Row, model};
use blackbox_feng::{NodeKind, runtime::pad};

fn shown(h: &Harness) -> std::sync::Arc<blackbox_feng::UiTree> {
    h.screens
        .presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard)
        .last()
        .unwrap()
        .tree
        .clone()
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_gauge_samples_are_visible_after_native_entrance_and_resume() {
    for backroom in [false, true] {
        for color in [false, true] {
            let mut model = model();
            model.shop = backroom;
            model.backroom = backroom;
            model.shop_marker_counts[13] = 1;
            model.gauge_has_turbo = true;
            model.gauge_max_rpm = 8250.0;
            model.draft.gauges.skin = 1;
            model.cars[0].parts.as_mut().unwrap().gauge_skins = vec![1];
            let page = match color {
                true => Page::GaugeColors { backroom },
                false => Page::Choices { row: Row::Gauge, backroom },
            };
            let mut h = Harness::open(page, model);
            for resume in [false, true] {
                if resume {
                    h.press(pad::BUTTON0);
                    assert_eq!(h.top(), "Showcase.fng");
                    h.press(pad::BACK);
                    assert_eq!(h.top().to_ascii_lowercase(), page.package().to_ascii_lowercase());
                }
                let tree = shown(&h);
                for hash in [0xc072_1eb9, 0x05d1_9f25, 0xd312_f0cb, 0xc62a_d685, 0x8fe2_a217] {
                    let node = tree.nodes.iter().find(|node| node.name_hash == hash).unwrap();
                    assert!(
                        node.visible && node.world_colour[3] > 0,
                        "{} gauge {hash:08x} after resume={resume}",
                        h.top()
                    );
                    let NodeKind::Image { texture, .. } = node.kind else { panic!("gauge sample is an image") };
                    assert!(h.screens.assets().texture(texture).is_some());
                }
            }
        }
    }
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_category_status_is_empty_and_unset_numbers_only_show_installed() {
    let h = Harness::open(Page::VinylGroups { backroom: false }, model());
    for hash in [0xd058_2feb, 0xcffb_7033] {
        let tree = shown(&h);
        assert!(!tree.nodes.iter().find(|node| node.name_hash == hash).unwrap().visible);
    }
    let h = Harness::open(Page::Numbers, model());
    let tree = shown(&h);
    let installed = tree.nodes.iter().find(|node| node.name_hash == 0xd058_2feb).unwrap();
    assert!(installed.visible);
    assert!(matches!(installed.kind, NodeKind::Image { texture: 0x696a_e039, .. }));
    assert!(!tree.nodes.iter().find(|node| node.name_hash == 0xcffb_7033).unwrap().visible);
    for hash in [0x2a08_ba92, 0x1a88_dc05] {
        assert_eq!(tree.nodes.iter().find(|node| node.name_hash == hash).unwrap().text.as_deref(), Some("-"));
    }
}
