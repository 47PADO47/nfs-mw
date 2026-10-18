//! `bb.ui` through the game's port: a fake mod registers a theme and overrides one slot, the theme is recorded, becomes a
//! `blackbox_ui` theme that builds the declared primitives for that slot, and falls back (`None`) for every other slot.

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_EXISTS, BB_ERR_NOT_FOUND, IFACE_UI};
use blackbox_modding::host::{UiBackend, UiPrim, UiRect, UiThemeSpec};
use blackbox_ui::{BuildCx, Component, DrawList, LayoutCx, NodeKind, Rect, Skin, Theme, ThemeId, justify};
use toml::Value;

use super::bridge::Bridge;
use super::interfaces;
use super::menu_store::ValueStore;
use super::support::MemoryStore;
use super::ui::{UiPort, any_theme, choice, chosen_display, step_choice, theme_id, themes};
use crate::ui::theme::ElementId;

fn setup() -> (Rc<Bridge>, UiPort) {
    let bridge = Rc::new(Bridge::with_store(Box::new(Rc::new(MemoryStore::default()))));
    (bridge.clone(), UiPort(bridge))
}

fn spec<'a>(name: &'a str, display: &'a str) -> UiThemeSpec<'a> {
    UiThemeSpec { name, display }
}

fn rect() -> UiRect {
    UiRect { x: 0.0, y: 0.0, w: 100.0, h: 50.0 }
}

/// The spec of a minimap override: a panel, a text and an image.
fn minimap_prims() -> Vec<UiPrim> {
    vec![
        UiPrim::Panel { rect: rect(), colour: [10, 20, 30, 255], texture: None },
        UiPrim::Text {
            rect: rect(),
            colour: [1, 2, 3, 255],
            justify: justify::H_CENTRE,
            font: "FONT_MW_BODY".into(),
            text: "SPEED".into(),
        },
        UiPrim::Image { rect: rect(), uv: [0.0, 0.0, 0.5, 1.0], texture: "Outrun_Backing".into() },
    ]
}

/// Lay a component out in the whole screen and draw it.
fn draw(mut component: Box<dyn Component>) -> DrawList {
    let skin = Skin::new();
    component.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    component.build(&mut out);
    out
}

#[test]
fn a_fake_mod_registers_a_theme_and_an_override_and_the_host_builds_it() {
    let (bridge, port) = setup();
    let handle = port.register_theme("example.a", &spec("night", "Night drive")).expect("the theme registers");
    port.set_slot("example.a", handle, ElementId::Minimap.raw(), &minimap_prims()).expect("the slot is set");

    // The theme is recorded, with the id the game derives from the mod and the name.
    let registered = themes(&bridge);
    assert_eq!(registered.len(), 1);
    assert_eq!(registered[0].id(), theme_id("example.a", "night"));
    assert!(matches!(registered[0].id(), ThemeId::Mod(_)), "a mod theme is a Mod id");
    assert_eq!(registered[0].name(), "night");

    // The overridden slot builds the declared primitives, in order.
    let component = registered[0].element(ElementId::Minimap.slot(), &BuildCx::new(&Skin::new())).expect("named");
    let out = draw(component);
    let nodes: Vec<_> = out.iter().collect();
    assert_eq!(nodes.len(), 3, "one node per primitive");
    assert_eq!(nodes[0].colour, [10, 20, 30, 255]);
    assert_eq!(nodes[1].text.as_deref(), Some("SPEED"));
    assert!(matches!(nodes[1].kind, NodeKind::Text { justification, .. } if justification == justify::H_CENTRE));
    assert!(matches!(nodes[2].kind, NodeKind::Image { uv: [0.0, 0.0, 0.5, 1.0], .. }));

    // Every other slot falls back: the theme returns None for it.
    assert!(registered[0].element(ElementId::Tachometer.slot(), &BuildCx::new(&Skin::new())).is_none());
}

#[test]
fn a_name_a_display_text_a_slot_and_a_primitive_are_checked_before_anything_is_recorded() {
    let (bridge, port) = setup();
    assert_eq!(port.register_theme("example.a", &spec("two words", "x")).map(|_| ()), Err(BB_ERR_ARG));
    assert_eq!(port.register_theme("example.a", &spec("night", "")).map(|_| ()), Err(BB_ERR_ARG));
    let handle = port.register_theme("example.a", &spec("night", "Night")).expect("registers");

    let refused = |slot: u32, prims: &[UiPrim]| port.set_slot("example.a", handle, slot, prims);
    assert_eq!(refused(9_999, &minimap_prims()), Err(BB_ERR_ARG), "a slot the catalogue does not list");
    assert_eq!(refused(ElementId::Minimap.raw(), &[]), Err(BB_ERR_ARG), "a slot with no primitives");
    let spaced_font =
        vec![UiPrim::Text { rect: rect(), colour: [0; 4], justify: 0, font: "A B".into(), text: "x".into() }];
    assert_eq!(refused(ElementId::Minimap.raw(), &spaced_font), Err(BB_ERR_ARG), "a font name with whitespace");
    let spaced_image = vec![UiPrim::Image { rect: rect(), uv: [0.0; 4], texture: "A B".into() }];
    assert_eq!(refused(ElementId::Minimap.raw(), &spaced_image), Err(BB_ERR_ARG), "a texture name with whitespace");
    assert_eq!(port.set_slot("example.a", 999, ElementId::Minimap.raw(), &minimap_prims()), Err(BB_ERR_NOT_FOUND));
    assert!(
        themes(&bridge)[0].element(ElementId::Minimap.slot(), &BuildCx::new(&Skin::new())).is_none(),
        "nothing set"
    );
}

#[test]
fn a_theme_name_is_unique_per_mod_but_another_mod_may_use_it() {
    let (bridge, port) = setup();
    port.register_theme("example.a", &spec("night", "Night")).expect("first");
    assert_eq!(port.register_theme("example.a", &spec("night", "Again")).map(|_| ()), Err(BB_ERR_EXISTS));
    port.register_theme("example.b", &spec("night", "Night")).expect("another mod may use the name");
    assert_eq!(themes(&bridge).len(), 2);
    assert_ne!(themes(&bridge)[0].id(), themes(&bridge)[1].id(), "the ids differ by owner");
}

#[test]
fn a_theme_is_removed_with_its_slots_and_takes_no_more_slots() {
    let (bridge, port) = setup();
    let handle = port.register_theme("example.a", &spec("night", "Night")).expect("registers");
    port.set_slot("example.a", handle, ElementId::Minimap.raw(), &minimap_prims()).expect("set");
    port.unregister_theme("example.a", handle).expect("removed");
    assert!(themes(&bridge).is_empty(), "the theme is gone");
    assert_eq!(port.set_slot("example.a", handle, ElementId::Minimap.raw(), &minimap_prims()), Err(BB_ERR_NOT_FOUND));
    assert_eq!(port.unregister_theme("example.a", handle), Err(BB_ERR_NOT_FOUND));
}

#[test]
fn a_theme_id_is_the_same_every_run_and_differs_by_owner_and_name() {
    assert_eq!(theme_id("example.a", "night"), theme_id("example.a", "night"));
    assert_ne!(theme_id("example.a", "night"), theme_id("example.a", "day"));
    assert_ne!(theme_id("example.a", "night"), theme_id("example.b", "night"));
}

#[test]
fn the_game_offers_bb_ui() {
    let game = interfaces();
    let offered: Vec<String> = game.registry.offers().into_iter().map(|offer| offer.id).collect();
    assert!(offered.iter().any(|id| id == IFACE_UI), "bb.ui is among the offers: {offered:?}");
}

/// A bridge whose saved values the test can read: the store is shared between the bridge and the test.
fn shared_setup() -> (Rc<MemoryStore>, Rc<Bridge>, UiPort) {
    let store = Rc::new(MemoryStore::default());
    let bridge = Rc::new(Bridge::with_store(Box::new(store.clone())));
    (store, bridge.clone(), UiPort(bridge))
}

#[test]
fn the_theme_choice_steps_through_the_built_in_style_and_each_mod_theme() {
    let (store, bridge, port) = shared_setup();
    port.register_theme("example.a", &spec("night", "Night drive")).expect("a registers");
    port.register_theme("example.b", &spec("day", "Day drive")).expect("b registers");
    let night = theme_id("example.a", "night");
    let day = theme_id("example.b", "day");
    assert!(any_theme(&bridge));
    assert_eq!(choice(&bridge), None, "the built-in style is chosen at first");
    assert_eq!(chosen_display(&bridge), None);

    step_choice(&bridge, true);
    assert_eq!((choice(&bridge), chosen_display(&bridge).as_deref()), (Some(night), Some("Night drive")));
    assert_eq!(store.saved_text("nfsmw", "ui_style_mod").as_deref(), Some("example.a/night"), "the choice is saved");

    step_choice(&bridge, true);
    assert_eq!(choice(&bridge), Some(day));
    step_choice(&bridge, true);
    assert_eq!(choice(&bridge), None, "forward past the last mod theme is the built-in style");
    assert_eq!(store.saved_text("nfsmw", "ui_style_mod").as_deref(), Some(""), "the built-in choice is saved as empty");
    step_choice(&bridge, false);
    assert_eq!(choice(&bridge), Some(day), "back from the built-in style is the last mod theme");
}

#[test]
fn a_saved_choice_selects_its_theme_again_when_the_theme_registers() {
    let (store, bridge, port) = shared_setup();
    store.save("nfsmw", "ui_style_mod", Value::String("example.a/night".into())).expect("saved");
    port.register_theme("example.b", &spec("day", "Day drive")).expect("b registers");
    assert_eq!(choice(&bridge), None, "another theme does not match the saved choice");
    port.register_theme("example.a", &spec("night", "Night drive")).expect("a registers");
    assert_eq!(choice(&bridge), Some(theme_id("example.a", "night")), "the saved theme is selected again");
    assert_eq!(chosen_display(&bridge).as_deref(), Some("Night drive"));
}

#[test]
fn a_saved_choice_for_an_absent_theme_leaves_the_built_in_style() {
    let (store, bridge, port) = shared_setup();
    store.save("nfsmw", "ui_style_mod", Value::String("example.z/gone".into())).expect("saved");
    port.register_theme("example.a", &spec("night", "Night drive")).expect("a registers");
    assert_eq!(choice(&bridge), None);
    assert_eq!(chosen_display(&bridge), None);
    assert!(any_theme(&bridge), "the theme row is still offered");
}

#[test]
fn the_built_in_choice_saved_as_empty_is_not_reselected() {
    let (store, bridge, port) = shared_setup();
    store.save("nfsmw", "ui_style_mod", Value::String(String::new())).expect("saved");
    port.register_theme("example.a", &spec("night", "Night drive")).expect("a registers");
    assert_eq!(choice(&bridge), None, "an empty saved choice is the built-in style");
}
