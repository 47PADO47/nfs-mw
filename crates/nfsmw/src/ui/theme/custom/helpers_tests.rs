//! Tests for the Custom building blocks the components share: the plate (`plate.rs`) and the text helpers (`text.rs`).
//! They check the nodes each emits against the token skin, with no game data and no GPU.

use blackbox_ui::{Component, DrawList, LayoutCx, NodeKind, Rect, Skin, UiNode, justify};

use super::layout::HudBox;
use super::plate::{PLATE_RADIUS, Plate, PlateStyle};
use super::text::{Label, Role, Sizing, size};
use super::tokens::{Colours, CustomLook, fade, keys, skin};
use crate::settings::hud::{HudPalette, HudSettings};
use crate::ui::present::Screen;
use crate::ui::raster_text::{CUSTOM_LABEL_WIDE_FONT_KEY, CUSTOM_NUMERIC_FONT_KEY, UNITS_PER_SCALE};

const GOLD: [u8; 4] = [255, 200, 80, 255];
const PLATE: Rect = Rect { x: 100.0, y: 50.0, w: 80.0, h: 30.0 };

fn token_skin(high_contrast: bool) -> Skin {
    skin(CustomLook { palette: HudPalette::Default, high_contrast }, GOLD)
}

fn built(component: &dyn Component) -> Vec<UiNode> {
    let mut out = DrawList::new();
    component.build(&mut out);
    out.iter().cloned().collect()
}

fn plate(style: PlateStyle, high_contrast: bool) -> Vec<UiNode> {
    let skin = token_skin(high_contrast);
    built(&Plate::new(PLATE, style, &LayoutCx::new(&skin, 0.0)))
}

#[test]
fn a_plate_is_rounded_quads_in_the_surface_colour_faded_by_its_opacity() {
    let nodes = plate(PlateStyle::SURFACE.with_opacity(0.5), false);
    assert!(nodes.len() > 1, "a rounded plate is several quads");
    let surface = fade(Colours::BRIEF.surface, 0.5);
    assert!(nodes.iter().all(|node| node.colour == surface), "every part is the faded surface");
    assert!(nodes.iter().all(|node| matches!(node.kind, NodeKind::Image { texture: 0, .. })));
    let raised = plate(PlateStyle::RAISED, false);
    assert!(raised.iter().all(|node| node.colour == Colours::BRIEF.surface_raised));
}

#[test]
fn the_accent_line_is_one_unit_along_the_top_between_the_corners() {
    let plain = plate(PlateStyle::SURFACE, false);
    let accented = plate(PlateStyle::SURFACE.with_accent(keys::ACCENT), false);
    assert_eq!(accented.len(), plain.len() + 1, "one more quad: the line");
    let line = accented.last().expect("the line is drawn last, over the fill");
    assert_eq!(line.colour, GOLD, "the accent is the theme's");
    let (w, h) = (line.world.x_axis.x, line.world.y_axis.y);
    assert!((h - 1.0).abs() < 1e-4, "one unit high, got {h}");
    assert!((w - (PLATE.w - 2.0 * PLATE_RADIUS)).abs() < 1e-4, "between the corners, got {w}");
    assert!((line.world.w_axis.y - (PLATE.min_y() + 0.5)).abs() < 1e-4, "on the plate's top edge");
    // At scale 2 the line is two units and the corners twice as round.
    let scaled = plate(PlateStyle::SURFACE.with_accent(keys::ACCENT).scaled(2.0), false);
    let line = scaled.last().expect("the line");
    assert!((line.world.y_axis.y - 2.0).abs() < 1e-4);
    assert!((line.world.x_axis.x - (PLATE.w - 4.0 * PLATE_RADIUS)).abs() < 1e-4);
}

#[test]
fn high_contrast_outlines_the_plate() {
    let plain = plate(PlateStyle::SURFACE, false);
    let outlined = plate(PlateStyle::SURFACE, true);
    let outline = [255, 255, 255, 153];
    assert_eq!(outlined.len(), plain.len() + 4, "four rules round the plate");
    assert_eq!(outlined.iter().filter(|node| node.colour == outline).count(), 4);
    let faded = plate(PlateStyle::SURFACE.with_opacity(0.5), true);
    assert_eq!(faded.iter().filter(|node| node.colour == fade(outline, 0.5)).count(), 4, "the outline fades too");
}

#[test]
fn a_numeric_label_is_one_text_node_in_the_numeric_face_sized_by_scale_and_text_size() {
    let skin = token_skin(false);
    let sizing = Sizing::new(1.5, 1.2);
    let text = Label::numeric("128", size::SPEED).justify(justify::H_RIGHT).laid_out(
        Rect::new(0.0, 0.0, 120.0, 60.0),
        sizing,
        &LayoutCx::new(&skin, 0.0),
    );
    let nodes = built(&text);
    assert_eq!(nodes.len(), 1);
    let node = &nodes[0];
    assert_eq!(node.text.as_deref(), Some("128"));
    assert!(matches!(node.kind, NodeKind::Text { font: CUSTOM_NUMERIC_FONT_KEY, justification: justify::H_RIGHT, .. }));
    assert_eq!(node.colour, Colours::BRIEF.text);
    let expected = size::SPEED * 1.5 * 1.2 / UNITS_PER_SCALE;
    assert!((node.world.x_axis.x - expected).abs() < 1e-4, "glyph scale {} for {expected}", node.world.x_axis.x);
    assert!((sizing.units(size::SPEED) - 66.6).abs() < 1e-3, "37 units at 1.5 and 1.2");
}

#[test]
fn a_heading_is_the_wide_face_in_the_secondary_colour_and_a_label_fades() {
    let skin = token_skin(false);
    let cx = LayoutCx::new(&skin, 0.0);
    let heading = built(&Label::heading("LAP", size::HEADING).laid_out(Rect::SCREEN, Sizing::default(), &cx));
    assert!(matches!(heading[0].kind, NodeKind::Text { font: CUSTOM_LABEL_WIDE_FONT_KEY, .. }));
    assert_eq!(heading[0].colour, Colours::BRIEF.text_secondary);
    let faded = Label::new(Role::Label, "Checkpoint", size::NOTIFICATION_TITLE).colour(keys::NAV).opacity(0.5);
    let nodes = built(&faded.laid_out(Rect::SCREEN, Sizing::default(), &cx));
    assert_eq!(nodes[0].colour, fade(Colours::BRIEF.nav, 0.5));
}

#[test]
fn the_sizing_follows_the_layout_scale_and_the_text_size_setting() {
    let screen = Screen { width: 1024.0, height: 768.0, pixels_per_point: 1.0 };
    let settings = HudSettings { scale: 1.5, text_size: 1.25, ..HudSettings::default() };
    let sizing = Sizing::of(&HudBox::new(screen, settings.scale), &settings);
    assert!((sizing.scale - 1.2).abs() < 1e-5, "1.5 times the 4:3 floor of 0.8");
    assert_eq!(sizing.text_size, 1.25);
    assert_eq!(Role::Numeric.font_key(), keys::FONT_NUMERIC);
}
