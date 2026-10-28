use super::*;

fn drawn_text(h: &Harness) -> Vec<String> {
    let shown = h.screens.presented(&crate::input::Bindings::default(), crate::input::InputDevice::Xbox);
    let tree = &shown.last().expect("garage screen").tree;
    tree.nodes
        .iter()
        .filter(|n| n.visible && n.world_colour[3] > 0 && tree.draw_order.contains(&n.index))
        .filter_map(|n| n.text.clone())
        .filter(|text| !text.is_empty())
        .collect()
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_playtest_garage_back_restores_header_footer_and_selection_repeatedly() {
    let mut h = Harness::open(Page::Main, model());
    let initial = drawn_text(&h);
    assert!(!initial.is_empty());
    assert!(initial.iter().any(|text| text == "Done"), "initial footer must exist");
    for _ in 0..3 {
        h.press(pad::ACCEPT);
        assert_eq!(h.top(), "CustomizeCategory.fng");
        let categories = drawn_text(&h);
        assert!(categories.iter().any(|text| text == "Done"));
        h.press(pad::ACCEPT);
        assert_eq!(h.top(), "CustomizeParts.fng");
        h.press(pad::BACK);
        assert_eq!(h.top(), "CustomizeCategory.fng");
        assert_eq!(drawn_text(&h), categories, "Back must restore the category's drawn text and footer");
        h.press(pad::BACK);
        assert_eq!(h.top(), "CustomizeMain.fng");
        assert_eq!(drawn_text(&h), initial, "Back must restore the root's drawn text and footer");
    }
    h.press(pad::RIGHT);
    let performance_root = drawn_text(&h);
    assert_ne!(performance_root, initial, "test a nondefault root selection");
    h.press(pad::ACCEPT);
    h.press(pad::RIGHT);
    let transmission_category = drawn_text(&h);
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "CustomizePerformance.fng");
    h.press(pad::BACK);
    assert_eq!(drawn_text(&h), transmission_category, "restore nondefault category selection");
    h.press(pad::BACK);
    assert_eq!(drawn_text(&h), performance_root);
    h.press(pad::BUTTON0);
    assert_eq!(h.top(), "Showcase.fng");
    h.press(pad::BACK);
    assert_eq!(h.top(), "CustomizeMain.fng");
    assert_eq!(drawn_text(&h), performance_root, "Showcase return keeps the selected root");
    assert!(h.saved.is_empty(), "return navigation must not save");
}
