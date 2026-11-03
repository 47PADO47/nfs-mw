use super::*;
use crate::frontend::garage_ui::Request;
use nfsmw_data::car::customization::Choice;

#[test]
#[ignore = "requires installed original garage packages"]
fn batch14_vinyl_back_preserves_each_parent_and_emits_no_menu_exit() {
    for backroom in [false, true] {
        let mut model = model();
        model.backroom = backroom;
        let catalog = model.cars[0].parts.as_mut().unwrap();
        catalog.choices[Part::Vinyl as usize].push(Choice {
            hash: 900,
            label: "Flames".into(),
            language_hash: None,
            carbon: false,
        });
        catalog.part_groups.insert(900, 0);
        let mut h = Harness::open(Page::Main, model);
        for page in [Page::Categories { section: Section::Visual, backroom }, Page::VinylGroups { backroom }] {
            h.ui.request(Request::Push(page));
            h.flush();
            h.run(0, 120);
        }
        for _ in 0..3 {
            h.ui.request(Request::Push(Page::Vinyls { group: 0, backroom }));
            h.flush();
            h.run(0, 120);
            h.commands.clear();
            h.run(pad::BACK, 180);
            assert_eq!(h.top(), Page::VinylGroups { backroom }.package());
            assert!(h.commands.is_empty(), "unexpected menu command: {:?}", h.commands);
            h.run(0, 90);
        }
        h.press(pad::BACK);
        assert_eq!(h.top(), Page::Categories { section: Section::Visual, backroom }.package());
        assert!(!h.events.iter().any(|event| matches!(event, MenuEvent::Cancel)));
        assert!(h.saved.is_empty());
    }
}

#[test]
#[ignore = "requires installed original garage packages"]
fn batch14_normal_route_vinyl_back_returns_to_visible_parent() {
    let mut model = model();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    catalog.choices[Part::Vinyl as usize].push(Choice {
        hash: 900,
        label: "Flames".into(),
        language_hash: None,
        carbon: false,
    });
    catalog.part_groups.insert(900, 0);
    let mut h = Harness::open(Page::Main, model);
    h.press(pad::RIGHT);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), Page::Categories { section: Section::Visual, backroom: false }.package());
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), Page::VinylGroups { backroom: false }.package());
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), Page::Vinyls { group: 0, backroom: false }.package());
    h.commands.clear();
    h.run(pad::BACK, 180);
    assert_eq!(h.top(), Page::VinylGroups { backroom: false }.package());
    assert!(h.commands.is_empty(), "unexpected menu command: {:?}", h.commands);
    let tree = h.screens.trees().pop().unwrap();
    assert!(
        tree.nodes
            .iter()
            .any(|n| n.visible && n.world_colour[3] > 0 && n.name_hash == crate::frontend::ids::TITLE_GROUP),
        "returned parent title is presented"
    );
    h.run(0, 90);
    h.press(pad::BACK);
    assert_eq!(h.top(), Page::Categories { section: Section::Visual, backroom: false }.package());
    assert!(!h.events.iter().any(|event| matches!(event, MenuEvent::Cancel)));
    assert!(h.saved.is_empty());
}
