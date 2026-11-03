use super::*;
use crate::input::{Bindings, InputDevice};
use blackbox_feng::{NodeKind, fe_hash_upper, font::TextStyle};
use glam::Vec3;

#[test]
#[ignore = "requires installed garage packages"]
fn batch16_gauge_footer_has_one_owner_on_both_devices_and_modes() {
    for backroom in [false, true] {
        for page in [Page::Choices { row: Row::Gauge, backroom }, Page::GaugeColors { backroom }] {
            let mut h = Harness::open(page, model());
            for _ in 0..2 {
                for device in [InputDevice::Keyboard, InputDevice::Xbox] {
                    let mut tree = h.screens.trees().pop().unwrap();
                    // A late authored SHOW must never bring a second footer owner back.
                    for index in 0..tree.nodes.len() {
                        let mut at = Some(index);
                        while let Some(parent) = at {
                            if [0x01bc_f3e9, 0x4170_d8cb].contains(&tree.nodes[parent].name_hash) {
                                tree.nodes[index].visible = true;
                                tree.nodes[index].world_colour[3] = 255;
                                if !tree.draw_order.contains(&index) {
                                    tree.draw_order.push(index);
                                }
                                break;
                            }
                            at = tree.nodes[parent].parent;
                        }
                    }
                    crate::frontend::prompts::decorate(
                        &mut tree,
                        page.package(),
                        crate::frontend::logic::Footer::GarageParts,
                        &Bindings::default(),
                        device,
                        h.screens.assets(),
                    );
                    for label in ["Back", "View Cart"] {
                        assert_eq!(
                            tree.nodes.iter().filter(|n| n.visible && n.text.as_deref() == Some(label)).count(),
                            1,
                            "duplicate {label} on {page:?} / {device:?}"
                        );
                    }
                    assert!(!tree.nodes.iter().any(|n| n.visible && [0x01bc_f3e9, 0x4170_d8cb].contains(&n.name_hash)));
                }
                h.run(0, 150);
            }
        }
    }
}

#[test]
#[ignore = "requires installed cart fonts and packages"]
fn batch16_cart_marks_align_with_rendered_text_centers() {
    let mut h = Harness::open(Page::Main, model());
    for part in [Part::Body, Part::Hood] {
        h.ui.preview(Row::Part(part), 1);
        h.ui.add_to_cart();
    }
    h.flush();
    h.ui.show_cart();
    h.flush();
    h.run(0, 150);
    for device in [InputDevice::Keyboard, InputDevice::Xbox] {
        let pages = h.screens.presented(&Bindings::default(), device);
        let tree = &pages.last().unwrap().tree;
        for row in 1..=2 {
            let node = |name: &str| tree.nodes.iter().find(|n| n.name_hash == fe_hash_upper(name)).unwrap();
            let label = node(&format!("PART_NAME_{row}"));
            let mark = node(&format!("CHECK_ICON_{row}"));
            assert!(label.visible && mark.visible);
            let NodeKind::Text { font, justification, leading, max_width } = label.kind else { panic!("cart label") };
            let assets = h.screens.assets();
            let font = assets.font(font).unwrap();
            let layout = font.layout_scaled(
                label.text.as_deref().unwrap(),
                TextStyle { justification, leading, max_width },
                assets.texture_size(assets.resolve(font.texture_hash)).unwrap(),
                assets.font_scale(font.hash),
            );
            let ys: Vec<_> = layout.quads.iter().flat_map(|q| [q.y0, q.y1]).collect();
            let top = ys.iter().copied().fold(f32::INFINITY, f32::min);
            let bottom = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let center = label.world.transform_point3(Vec3::new(0.0, (top + bottom) * 0.5, 0.0));
            let icon = mark.world.transform_point3(Vec3::ZERO);
            assert!(
                (center.y - icon.y).abs() < 1.0,
                "cart mark must align with rendered glyphs: {center:?} / {icon:?}"
            );
        }
    }
}

fn saved() -> crate::garage::Model {
    let mut m = model();
    m.custom_slots_enabled = true;
    m.custom_slots.push(("saved".into(), "carrera_gt".into()));
    m.custom_slot_cars.insert("saved".into(), m.draft.car.clone());
    m.custom_slot = Some("saved".into());
    m
}

#[test]
#[ignore = "requires installed manager packages and art"]
fn batch16_manager_uses_live_display_name_and_never_placeholder_model_art() {
    let mut m = saved();
    m.cars[0].name = "Porsche Carrera GT".into();
    m.cars[0].manufacturer = "Porsche".into();
    m.cars[0].physics_name = "carrera_gt".into();
    m.cars[0].folder = "CARRERAGT".into();
    m.draft.car = "CARRERAGT".into();
    m.applied.car = "CARRERAGT".into();
    m.custom_slot_cars.insert("saved".into(), "CARRERAGT".into());
    let h = Harness::open(Page::Manager, m);
    let tree = h.screens.trees().pop().unwrap();
    assert!(tree.nodes.iter().any(|n| n.visible && n.text.as_deref() == Some("Porsche Carrera GT")));
    let logo = tree.nodes.iter().find(|n| n.name_hash == 0xb05d_d708).unwrap();
    assert!(matches!(logo.kind, NodeKind::Image { texture, .. } if texture != fe_hash_upper("GENERIC_LOGO_256")));
}

#[test]
#[ignore = "requires installed manager and customization packages"]
fn batch16_manager_returns_with_its_grid_and_footer_after_customization() {
    let mut h = Harness::open(Page::Manager, saved());
    for _ in 0..3 {
        h.press(pad::ACCEPT);
        assert_eq!(h.top(), Page::Main.package());
        h.press(pad::ACCEPT);
        h.press(pad::ACCEPT);
        h.press(pad::BACK);
        h.press(pad::BACK);
        h.press(pad::BACK);
        h.run(0, 150);
        assert_eq!(h.top(), Page::Manager.package());
        for device in [InputDevice::Xbox, InputDevice::Keyboard] {
            let pages = h.screens.presented(&Bindings::default(), device);
            let tree = &pages.last().unwrap().tree;
            assert!(
                tree.nodes
                    .iter()
                    .any(|n| n.name_hash == fe_hash_upper("CAR_ICON_1") && n.visible && n.world_colour[3] > 0),
                "manager grid must return"
            );
            assert!(tree.nodes.iter().any(|n| n.visible && n.text.as_deref() == Some("Customize Car")));
        }
    }
}
