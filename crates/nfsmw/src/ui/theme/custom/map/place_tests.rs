//! Tests for the map's placement (`place.rs`): at the top centre with a little padding, clear of the top columns on the
//! four window shapes, in every mode, and the stack of top-centre elements under it.

use blackbox_ui::Rect;
use glam::Vec2;

use super::modes::{CENTERED_RADII, COMPACT_RADII, EXPANDED_RADII, Overlay};
use super::place::{MIN_HALF_WIDTH, SIDE_RESERVE, TOP_GAP, centre, cleared, radii, top_clearance};
use crate::settings::hud::{HudSettings, MapMode};
use crate::ui::present::Screen;
use crate::ui::theme::custom::layout::{Anchor, HudBox, SAFE_INSET, window};

const FOUR_THREE: Screen = Screen { width: 1024.0, height: 768.0, pixels_per_point: 1.0 };
const SIXTEEN_NINE: Screen = Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 };
const TWENTY_ONE_NINE: Screen = Screen { width: 2560.0, height: 1080.0, pixels_per_point: 1.0 };
const THIRTY_TWO_NINE: Screen = Screen { width: 5120.0, height: 1440.0, pixels_per_point: 1.25 };
const SHAPES: [Screen; 4] = [FOUR_THREE, SIXTEEN_NINE, TWENTY_ONE_NINE, THIRTY_TWO_NINE];
const MODES: [(MapMode, [f32; 2]); 3] =
    [(MapMode::Compact, COMPACT_RADII), (MapMode::Centered, CENTERED_RADII), (MapMode::Expanded, EXPANDED_RADII)];

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

/// The ellipse's bounding box: its centre and twice its half-axes.
fn bounds(frame: &HudBox, fractions: [f32; 2]) -> Rect {
    let r = radii(fractions, frame);
    let c = centre(frame, r);
    Rect::new(c.x, c.y, 2.0 * r.x, 2.0 * r.y)
}

fn settings(mode: MapMode) -> HudSettings {
    HudSettings { map_mode: mode, ..HudSettings::default() }
}

#[test]
fn the_map_hangs_from_the_top_with_only_the_safe_padding_and_is_centred_across() {
    for shape in SHAPES {
        let frame = HudBox::new(shape, 1.0);
        for (_, fractions) in MODES {
            let b = bounds(&frame, fractions);
            let top_padding = b.min_y() - window(shape).min_y();
            assert!(near(top_padding, SAFE_INSET * 480.0), "{top_padding} below the screen's top on {shape:?}");
            assert!(top_padding <= 0.05 * 480.0, "at most a few percent of the screen's height");
            assert!(near(b.x, frame.content().x), "centred across on {shape:?}");
        }
    }
}

#[test]
fn every_mode_ends_in_the_upper_part_of_the_screen_and_a_bigger_one_grows_downwards() {
    for shape in SHAPES {
        let frame = HudBox::new(shape, 1.0);
        let mut last_bottom = f32::NEG_INFINITY;
        for (mode, fractions) in MODES {
            let b = bounds(&frame, fractions);
            // The upper 40 % of the 480-unit screen ends 48 units above the centre.
            assert!(b.max_y() < -48.0, "{mode:?} stays in the upper part of the screen on {shape:?}: {}", b.max_y());
            assert!(b.max_y() >= last_bottom, "{mode:?} is no shorter than the mode before");
            last_bottom = b.max_y();
        }
    }
}

#[test]
fn the_map_stays_inside_the_window_and_the_safe_area() {
    for shape in SHAPES {
        for hud_scale in [0.75, 1.0, 1.5] {
            let frame = HudBox::new(shape, hud_scale);
            for (_, fractions) in MODES {
                let b = bounds(&frame, fractions);
                assert!(b.min_x() >= frame.safe().min_x() && b.max_x() <= frame.safe().max_x(), "{shape:?}");
                assert!(b.min_y() >= frame.safe().min_y() - 1e-3 && b.max_y() <= frame.safe().max_y(), "{shape:?}");
            }
        }
    }
}

#[test]
fn the_map_clears_the_top_columns_where_they_leave_room() {
    for shape in SHAPES {
        for hud_scale in [0.75, 1.0] {
            let frame = HudBox::new(shape, hud_scale);
            let column_edge = frame.content().max_x() - (SAFE_INSET * 480.0) - SIDE_RESERVE * frame.scale();
            for (_, fractions) in MODES {
                let b = bounds(&frame, fractions);
                assert!(
                    b.max_x() < column_edge,
                    "{shape:?} at {hud_scale}: {} reaches the column at {column_edge}",
                    b.max_x()
                );
                assert!(b.min_x() > -column_edge);
            }
        }
    }
}

#[test]
fn a_big_hud_scale_squeezes_the_map_to_a_floor_not_to_nothing() {
    let frame = HudBox::new(SIXTEEN_NINE, 1.5);
    let squeezed = radii(EXPANDED_RADII, &frame);
    assert!(near(squeezed.x, MIN_HALF_WIDTH), "{squeezed:?}");
    assert!(near(squeezed.y / squeezed.x, 0.5), "the shape is kept: {squeezed:?}");
}

#[test]
fn an_unconstrained_window_gets_the_modes_own_widths() {
    let frame = HudBox::new(SIXTEEN_NINE, 1.0);
    for (_, fractions) in MODES {
        let r = radii(fractions, &frame);
        assert!(near(r.x, fractions[0] * 480.0) && near(r.y, fractions[1] * 480.0), "{r:?}");
    }
    assert_eq!(radii([0.0, 0.0], &frame), Vec2::ZERO);
}

#[test]
fn the_top_centre_is_cleared_by_the_maps_height_and_a_gap_and_only_with_the_map_on() {
    let frame = HudBox::new(SIXTEEN_NINE, 1.0);
    assert_eq!(top_clearance(&settings(MapMode::Off), &frame), 0.0);
    for (mode, fractions) in MODES {
        let expected = 2.0 * radii(fractions, &frame).y + TOP_GAP;
        assert!(near(top_clearance(&settings(mode), &frame), expected), "{mode:?}");
    }
    assert!(Overlay::of(MapMode::Off, 0.35).shown == 0.0);
}

#[test]
fn what_stacks_under_the_map_starts_below_it() {
    let strip = (300.0, 24.0);
    for shape in SHAPES {
        let plain = HudBox::new(shape, 1.0);
        for (mode, fractions) in MODES {
            let frame = cleared(plain, &settings(mode));
            let map_bottom = bounds(&plain, fractions).max_y();
            let rect = frame.place(Anchor::TopCentre, (0.0, 0.0), strip);
            assert!(
                near(rect.min_y(), map_bottom + TOP_GAP),
                "{mode:?} on {shape:?}: {} under {map_bottom}",
                rect.min_y()
            );
        }
    }
}

#[test]
fn the_clearance_moves_only_top_centre_elements() {
    let plain = HudBox::new(SIXTEEN_NINE, 1.0);
    let frame = cleared(plain, &settings(MapMode::Expanded));
    for anchor in [Anchor::TopLeft, Anchor::TopRight, Anchor::BottomLeft, Anchor::BottomRight] {
        assert_eq!(frame.place(anchor, (3.0, 4.0), (100.0, 30.0)), plain.place(anchor, (3.0, 4.0), (100.0, 30.0)));
    }
    assert_eq!(cleared(plain, &settings(MapMode::Off)), plain, "no map, no clearance");
}

#[test]
fn a_card_for_the_upper_third_is_pushed_down_only_as_far_as_the_map_needs() {
    let plain = HudBox::new(SIXTEEN_NINE, 1.0);
    // The countdown's box: 70 high, centred 96 units down: its top is 61 units below the safe area's top.
    let size = (120.0, 70.0);
    let offset = (0.0, 96.0 - size.1 / 2.0);
    let at = |mode| cleared(plain, &settings(mode)).place_clear(Anchor::TopCentre, offset, size);
    let free = plain.place(Anchor::TopCentre, offset, size);
    assert_eq!(at(MapMode::Off), free, "no map: where it always was");
    let expanded = at(MapMode::Expanded);
    let band = plain.safe().min_y() + top_clearance(&settings(MapMode::Expanded), &plain);
    assert!(near(expanded.min_y(), band), "Expanded pushes it to the map's edge: {} and {band}", expanded.min_y());
    let compact = at(MapMode::Compact);
    assert!(compact.min_y() >= free.min_y(), "never moved up");
    assert!(compact.min_y() < expanded.min_y(), "a smaller map pushes it less");
    let low = plain.place_clear(Anchor::TopCentre, (0.0, 300.0), size);
    let low_cleared = cleared(plain, &settings(MapMode::Expanded)).place_clear(Anchor::TopCentre, (0.0, 300.0), size);
    assert_eq!(low, low_cleared, "a card already below the map stays");
}
