//! The native five-by-two manager scrolls by rows and shares its window with pointer slots.

use super::{Harness, Page, model};
use crate::garage::Selection;
use blackbox_feng::{NodeKind, fe_hash_upper, runtime::pad};

/// Sample the package's own focus response: the selected marker is a separate native cursor.
fn native_cursor(focused: u32) -> blackbox_feng::UiNode {
    use crate::ui::{Catalog, SCREEN_FILES};
    use game_install::GameDir;
    static PACKAGE: std::sync::OnceLock<blackbox_feng::Package> = std::sync::OnceLock::new();
    let package = PACKAGE.get_or_init(|| {
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        Catalog::load(&dir, &SCREEN_FILES).find("MyCarsManager.fng").unwrap().clone()
    });
    let mut runtime = crate::ui::runtime();
    let id = runtime.load(package.clone());
    runtime.update(2.0);
    let object = runtime.find_guid(id, focused).unwrap();
    runtime.focus_button(object);
    runtime.update(1.0 / 60.0);
    runtime.update(1.0 / 60.0);
    runtime.tree(id).nodes.into_iter().find(|node| node.name_hash == 0x0674_5352).unwrap()
}

fn selection(index: usize) -> Selection {
    let car = match index % 2 {
        0 => "BMWM3GTR",
        _ => "MUSTANGGT",
    };
    Selection { car: car.into(), color: Some([index as u8, 80, 160]), ..Default::default() }
}

fn window(h: &Harness, first: usize, selected: usize) {
    let tree = h
        .screens
        .presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard)
        .last()
        .unwrap()
        .tree
        .clone();
    let focused = h.screens.focused_button().expect("one native manager tile takes focus");
    let mut selected_guid = None;
    for slot in 0..10 {
        let hash = fe_hash_upper(&format!("CAR_ICON_{}", slot + 1));
        let node = tree.nodes.iter().find(|node| node.name_hash == hash).unwrap();
        let index = first + slot;
        assert_eq!(node.visible, index < 14, "native tile {} for data index {index}", slot + 1);
        if index >= 14 {
            continue;
        }
        let texture = match index {
            13 => 0x91d3_a6ba,
            _ if index.is_multiple_of(2) => fe_hash_upper("CARSELECT_MANUFACTURER_BMW"),
            _ => fe_hash_upper("CARSELECT_MANUFACTURER_FORD"),
        };
        assert!(matches!(node.kind, NodeKind::Image { texture: actual, .. } if actual == texture));
        assert_eq!(node.index == focused.index, index == selected, "selected native tile {index}");
        if index == selected {
            assert!(node.world_colour[3] > 0 && tree.draw_order.contains(&node.index));
            selected_guid = Some(node.guid);
        }
    }
    let cursor = tree.nodes.iter().find(|node| node.name_hash == 0x0674_5352).unwrap();
    let expected = native_cursor(selected_guid.expect("the selected tile is in the visible window"));
    assert!(cursor.visible && cursor.world_colour[3] > 0, "the native selection marker remains visible");
    assert_eq!(cursor.position, expected.position, "the authored cursor tracks the visible selected tile");
    assert_eq!(cursor.colour, expected.colour, "selection keeps the native cursor color");
    assert!(
        tree.draw_order.iter().any(|&index| {
            let mut at = Some(index);
            for _ in 0..tree.nodes.len() {
                let Some(index) = at else { break };
                if index == cursor.index {
                    return true;
                }
                at = tree.nodes[index].parent;
            }
            false
        }),
        "the native selection marker has actual drawn artwork"
    );
    let heading = tree.nodes.iter().find(|node| node.name_hash == 0x5e7b_09c9).unwrap();
    assert_eq!(heading.text.as_deref(), Some(format!("Saved {selected}").as_str()));
    assert_eq!(h.model.custom_slot.as_deref(), Some(format!("saved{selected}").as_str()));
    assert_eq!(h.model.draft, selection(selected), "the highlighted tile must match the current preview");
    let backing = tree.nodes.iter().find(|node| node.name_hash == 0x688a_1551).unwrap();
    let handle = tree.nodes.iter().find(|node| node.name_hash == 0x3fbb_7a4e).unwrap();
    assert!(backing.visible && handle.visible, "three data rows require the native scrollbar");
    let height = backing.size.y * 2.0 / 3.0;
    let top = backing.local_position.y - backing.size.y * 0.5 + (first / 5) as f32 * (backing.size.y - height);
    assert!((handle.size.y - height).abs() < 0.001, "native handle extent counts data rows");
    assert!((handle.local_position.y - height * 0.5 - top).abs() < 0.001, "native handle tracks the first visible row");
}

fn fixture() -> Harness {
    let mut model = model();
    model.custom_slots_enabled = true;
    let mut ford = model.cars[0].clone();
    ford.folder = "MUSTANGGT".into();
    ford.name = "Mustang GT".into();
    ford.physics_name = "mustanggt".into();
    ford.manufacturer = "Ford".into();
    model.cars.push(ford);
    for index in 0..13 {
        let id = format!("saved{index}");
        model.custom_slots.push((id.clone(), format!("Saved {index}")));
        model.custom_slot_cars.insert(id, selection(index).car);
    }
    model.custom_slot = Some("saved11".into());
    model.draft = selection(11);
    model.applied = model.draft.clone();
    let mut h = Harness::open(Page::Manager, model);
    h.slot_selections.extend((0..13).map(|index| (format!("saved{index}"), selection(index))));
    h
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_manager_window_scrolls_one_row_and_matches_initial_slot_preview_pointer_and_scrollbar() {
    let mut h = fixture();
    window(&h, 5, 11);
    h.press(pad::BUTTON0);
    assert_eq!(h.top(), "Showcase.fng");
    h.press(pad::BACK);
    window(&h, 5, 11);
    h.press(pad::UP);
    window(&h, 5, 6);
    h.press(pad::UP);
    window(&h, 0, 1);
    h.press(pad::DOWN);
    window(&h, 0, 6);
    h.press(pad::DOWN);
    window(&h, 5, 11);
    h.click_named(fe_hash_upper("CAR_ICON_1"));
    window(&h, 5, 5);
    h.click_named(fe_hash_upper("scrollbar_Arrow_1"));
    window(&h, 0, 0);
    h.click_named(fe_hash_upper("scrollbar_Arrow_2"));
    window(&h, 0, 5);
    h.click_named(fe_hash_upper("scrollbar_Arrow_2"));
    window(&h, 5, 10);
    assert!(h.saved.is_empty());
    assert_eq!(h.model.custom_slots.len(), 13);
}

fn backing_point(h: &Harness, above: bool) -> glam::Vec2 {
    let tree = h
        .screens
        .presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard)
        .last()
        .unwrap()
        .tree
        .clone();
    let backing = tree.nodes.iter().find(|node| node.name_hash == 0x688a_1551).unwrap();
    let handle = tree.nodes.iter().find(|node| node.name_hash == 0x3fbb_7a4e).unwrap();
    assert!(backing.visible && handle.visible);
    let (lower, upper) = match above {
        true => (
            backing.world.transform_point3(glam::Vec3::new(-0.5, -0.5, 0.0)).y,
            handle.world.transform_point3(glam::Vec3::new(-0.5, -0.5, 0.0)).y,
        ),
        false => (
            handle.world.transform_point3(glam::Vec3::new(0.5, 0.5, 0.0)).y,
            backing.world.transform_point3(glam::Vec3::new(0.5, 0.5, 0.0)).y,
        ),
    };
    assert!(upper > lower, "the actual native backing must have exposed space on the requested side");
    glam::Vec2::new(backing.world.transform_point3(glam::Vec3::ZERO).x, (lower + upper) * 0.5)
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_manager_backing_clicks_move_one_row_and_thumb_click_opens_the_current_car() {
    let mut h = fixture();
    window(&h, 5, 11);
    h.click_point(backing_point(&h, true));
    window(&h, 5, 6);
    h.click_point(backing_point(&h, true));
    window(&h, 0, 1);
    h.click_point(backing_point(&h, false));
    window(&h, 0, 6);
    h.click_point(backing_point(&h, false));
    window(&h, 5, 11);
    h.click_named(fe_hash_upper("scrollbar_Handle"));
    assert_eq!(h.top(), "CustomizeMain.fng");
    assert_eq!(h.model.custom_slot.as_deref(), Some("saved11"));
    assert_eq!(h.model.draft, selection(11));
    assert_eq!(h.model.custom_slots.len(), 13);
    assert!(h.saved.is_empty());
}
