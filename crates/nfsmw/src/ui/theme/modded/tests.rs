//! Mod themes: a slot's primitives build into the right nodes, a slot the mod did not name falls back, and the registry
//! selects a mod theme and gives it up again. No game data and no GPU are needed.

use blackbox_feng::resource_handle;
use blackbox_modding::host::{UiPrim, UiRect};
use blackbox_ui::{BuildCx, Component, DrawList, LayoutCx, NodeKind, Rect, Skin, SlotId, Theme, ThemeId, justify};

use super::ModTheme;
use crate::settings::UiStyle;
use crate::ui::theme::{ElementId, UiThemes};

const MOD: ThemeId = ThemeId::Mod(7);

fn rect() -> UiRect {
    UiRect { x: 0.0, y: 0.0, w: 100.0, h: 50.0 }
}

/// A theme with one overridden slot, the minimap, whose spec is a panel, a text and an image.
fn minimap_theme() -> ModTheme {
    let mut theme = ModTheme::new(MOD, "night", "Night drive");
    theme.set_slot(
        ElementId::Minimap.slot(),
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
        ],
    );
    theme
}

/// Lay `component` out in the whole screen and draw it.
fn draw(mut component: Box<dyn Component>) -> DrawList {
    let skin = Skin::new();
    component.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    component.build(&mut out);
    out
}

#[test]
fn a_theme_is_a_blackbox_ui_theme_with_its_id() {
    assert_eq!(minimap_theme().id(), MOD);
}

#[test]
fn an_overridden_slot_builds_its_primitives_in_order() {
    let theme = minimap_theme();
    let component = theme.element(ElementId::Minimap.slot(), &BuildCx::new(&Skin::new())).expect("the slot is named");
    let out = draw(component);
    assert_eq!(out.len(), 3);
    let nodes: Vec<_> = out.iter().collect();
    assert!(matches!(nodes[0].kind, NodeKind::Image { texture: 0, .. }));
    assert_eq!(nodes[0].colour, [10, 20, 30, 255]);
    assert_eq!(
        nodes[1].kind,
        NodeKind::Text {
            font: resource_handle("FONT_MW_BODY"),
            justification: justify::H_CENTRE,
            leading: 0,
            max_width: 100
        }
    );
    assert_eq!((nodes[1].text.as_deref(), nodes[1].colour), (Some("SPEED"), [1, 2, 3, 255]));
    assert!(matches!(
        nodes[2].kind,
        NodeKind::Image { texture, uv: [0.0, 0.0, 0.5, 1.0], .. } if texture == resource_handle("Outrun_Backing")
    ));
}

#[test]
fn a_textured_panel_carries_its_texture_handle() {
    let mut theme = ModTheme::new(MOD, "night", "Night");
    let prim = UiPrim::Panel { rect: rect(), colour: [0, 0, 0, 255], texture: Some("Panel_Back".into()) };
    theme.set_slot(ElementId::Blips.slot(), vec![prim]);
    let out = draw(theme.element(ElementId::Blips.slot(), &BuildCx::new(&Skin::new())).expect("named"));
    let node = out.iter().next().expect("one node");
    assert!(matches!(node.kind, NodeKind::Image { texture, .. } if texture == resource_handle("Panel_Back")));
}

#[test]
fn a_slot_the_mod_did_not_name_is_none() {
    let theme = minimap_theme();
    assert!(theme.element(ElementId::Tachometer.slot(), &BuildCx::new(&Skin::new())).is_none());
    let unknown = theme.element(SlotId::new(9_999), &BuildCx::new(&Skin::new()));
    assert!(unknown.is_none(), "a slot number the catalogue does not list is not a mod's to draw");
}

#[test]
fn a_second_spec_for_a_slot_replaces_the_first() {
    let mut theme = minimap_theme();
    theme.set_slot(ElementId::Minimap.slot(), vec![UiPrim::Panel { rect: rect(), colour: [9; 4], texture: None }]);
    let out = draw(theme.element(ElementId::Minimap.slot(), &BuildCx::new(&Skin::new())).expect("named"));
    assert_eq!(out.len(), 1);
}

#[test]
fn the_registry_selects_a_mod_theme_and_gives_it_up_for_original_again() {
    let mut themes = UiThemes::build();
    let before = draw(themes.resolve(ElementId::Minimap));
    themes.set_mod_themes(vec![minimap_theme()]);
    themes.select_mod(MOD).expect("the mod theme is registered");
    assert_eq!(themes.themes.active(), MOD);
    assert_eq!(draw(themes.resolve(ElementId::Minimap)).len(), 3, "the mod's spec draws");
    assert_eq!(draw(themes.resolve(ElementId::Tachometer)).len(), before.len(), "unnamed slots keep the Original look");

    themes.clear_mod();
    assert_eq!(themes.themes.active(), ThemeId::Original);
    assert_eq!(draw(themes.resolve(ElementId::Minimap)).len(), before.len(), "back to the Original minimap");
}

#[test]
fn a_chosen_mod_theme_survives_the_style_setting_until_cleared() {
    let mut themes = UiThemes::build();
    themes.set_mod_themes(vec![minimap_theme()]);
    themes.select_mod(MOD).expect("registered");
    themes.select(UiStyle::Custom);
    assert_eq!(themes.themes.active(), MOD, "the per-frame style sync does not undo the mod choice");
    themes.clear_mod();
    assert_eq!(themes.themes.active(), ThemeId::Custom);
}

#[test]
fn selecting_an_unregistered_mod_theme_is_refused_and_removing_one_drops_the_choice() {
    let mut themes = UiThemes::build();
    assert!(themes.select_mod(MOD).is_err(), "no mod registered it");
    themes.set_mod_themes(vec![minimap_theme()]);
    themes.select_mod(MOD).expect("registered");
    themes.set_mod_themes(Vec::new());
    assert_eq!(themes.themes.active(), ThemeId::Original, "the theme is gone, so Original is active");
    themes.set_mod_themes(vec![minimap_theme()]);
    assert_eq!(themes.themes.active(), ThemeId::Original, "the choice was dropped with the theme");
}

#[test]
fn a_mod_theme_reports_the_right_rectangle_for_its_primitives() {
    // The rectangle of a primitive becomes the component's rectangle, so the node sits at the declared centre and size.
    let mut theme = ModTheme::new(MOD, "night", "Night");
    let at = UiRect { x: 12.0, y: -8.0, w: 40.0, h: 20.0 };
    theme.set_slot(ElementId::Blips.slot(), vec![UiPrim::Panel { rect: at, colour: [1; 4], texture: None }]);
    let out = draw(theme.element(ElementId::Blips.slot(), &BuildCx::new(&Skin::new())).expect("named"));
    let node = out.iter().next().expect("one node");
    assert_eq!((node.world.w_axis.x, node.world.w_axis.y), (12.0, -8.0));
    assert_eq!((node.world.x_axis.x, node.world.y_axis.y), (40.0, 20.0));
}
