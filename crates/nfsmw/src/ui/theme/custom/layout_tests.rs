//! Tests for the Custom HUD layout (`layout.rs`) on the four window shapes the spec names: 4:3, 16:9, 21:9 and 32:9.

use blackbox_ui::Rect;

use super::layout::{Anchor, CANVAS, HudBox, MAX_ASPECT, NARROW_SCALE_FLOOR, window};
use crate::ui::present::Screen;

const FOUR_THREE: Screen = Screen { width: 1024.0, height: 768.0, pixels_per_point: 1.0 };
const SIXTEEN_NINE: Screen = Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 };
const TWENTY_ONE_NINE: Screen = Screen { width: 2560.0, height: 1080.0, pixels_per_point: 1.0 };
const THIRTY_TWO_NINE: Screen = Screen { width: 5120.0, height: 1440.0, pixels_per_point: 1.25 };
const SHAPES: [Screen; 4] = [FOUR_THREE, SIXTEEN_NINE, TWENTY_ONE_NINE, THIRTY_TWO_NINE];

/// Half the 16:9 box's width in canvas units: 240 times 16/9.
const HALF_16_9: f32 = 240.0 * MAX_ASPECT;
/// The safe inset: 3.5 % of the 480-unit height.
const INSET: f32 = 16.8;
/// The size of a sample element, at scale 1.
const ELEMENT: (f32, f32) = (120.0, 40.0);

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

/// Whether `inner` lies inside `outer`, edges included, to within rounding.
fn inside(inner: Rect, outer: Rect) -> bool {
    inner.min_x() >= outer.min_x() - 1e-3
        && inner.max_x() <= outer.max_x() + 1e-3
        && inner.min_y() >= outer.min_y() - 1e-3
        && inner.max_y() <= outer.max_y() + 1e-3
}

#[test]
fn the_content_box_is_the_window_up_to_16_9_and_the_central_16_9_beyond() {
    assert!(near(HudBox::new(FOUR_THREE, 1.0).content().w, 640.0), "4:3 is the whole window");
    assert!(near(HudBox::new(SIXTEEN_NINE, 1.0).content().w, 2.0 * HALF_16_9), "16:9 is the whole window");
    for wide in [TWENTY_ONE_NINE, THIRTY_TWO_NINE] {
        let frame = HudBox::new(wide, 1.0);
        assert!(near(frame.content().w, 2.0 * HALF_16_9), "an ultrawide box is 16:9, not {}", frame.content().w);
        assert!(frame.content().w < window(wide).w, "the box is narrower than the ultrawide window");
        assert!(near(frame.content().x, 0.0), "the box is centred");
    }
    for shape in SHAPES {
        assert!(near(HudBox::new(shape, 1.0).content().h, 480.0), "the box is as tall as the window");
    }
}

#[test]
fn every_anchor_places_its_element_inside_the_window_and_the_safe_area() {
    for shape in SHAPES {
        for hud_scale in [0.75, 1.0, 1.5] {
            let frame = HudBox::new(shape, hud_scale);
            for anchor in Anchor::ALL {
                let rect = frame.place(anchor, (0.0, 0.0), ELEMENT);
                assert!(inside(rect, frame.safe()), "{anchor:?} at {hud_scale} leaves the safe area on {shape:?}");
                assert!(inside(rect, window(shape)), "{anchor:?} at {hud_scale} leaves the window on {shape:?}");
            }
        }
    }
}

#[test]
fn on_ultrawide_windows_the_cluster_anchors_to_the_16_9_box_not_the_window() {
    for wide in [TWENTY_ONE_NINE, THIRTY_TWO_NINE] {
        let frame = HudBox::new(wide, 1.0);
        let cluster = frame.place(Anchor::BottomRight, (0.0, 0.0), ELEMENT);
        assert!(near(cluster.max_x(), HALF_16_9 - INSET), "right edge {} on {wide:?}", cluster.max_x());
        assert!(near(cluster.max_y(), 240.0 - INSET));
        assert!(cluster.max_x() < window(wide).max_x() - 100.0, "the cluster is not at the window's corner");
        let left = frame.place(Anchor::TopLeft, (0.0, 0.0), ELEMENT);
        assert!(near(left.min_x(), -HALF_16_9 + INSET), "left edge {} on {wide:?}", left.min_x());
    }
}

#[test]
fn the_scale_grows_an_element_about_its_anchor() {
    let small = HudBox::new(SIXTEEN_NINE, 1.0);
    let large = HudBox::new(SIXTEEN_NINE, 1.5);
    let corner = |frame: &HudBox| frame.place(Anchor::BottomRight, (0.0, 0.0), ELEMENT);
    let (a, b) = (corner(&small), corner(&large));
    assert!(near(a.max_x(), b.max_x()) && near(a.max_y(), b.max_y()), "the anchored corner stays put");
    assert!(near(b.w, 1.5 * a.w) && near(b.h, 1.5 * a.h), "the element is half as large again");
    // A centred element keeps its centre.
    let top = |frame: &HudBox| frame.place(Anchor::TopCentre, (0.0, 0.0), ELEMENT);
    assert!(near(top(&small).x, top(&large).x) && near(top(&small).min_y(), top(&large).min_y()));
    // The offset is in units at scale 1, so it scales too.
    let moved = large.place(Anchor::BottomRight, (-10.0, -4.0), ELEMENT);
    assert!(near(moved.max_x(), b.max_x() - 15.0) && near(moved.max_y(), b.max_y() - 6.0));
}

#[test]
fn the_safe_inset_is_the_same_at_every_scale() {
    for hud_scale in [0.75, 1.0, 1.5] {
        let frame = HudBox::new(SIXTEEN_NINE, hud_scale);
        assert!(near(frame.inset(), INSET));
        assert!(near(frame.safe().max_x(), frame.content().max_x() - INSET));
        assert!(near(frame.safe().min_y(), frame.content().min_y() + INSET));
    }
}

#[test]
fn a_narrow_window_shrinks_the_hud_to_the_floor() {
    assert!(near(HudBox::new(FOUR_THREE, 1.0).scale(), NARROW_SCALE_FLOOR), "4:3 draws at 0.8");
    assert!(near(HudBox::new(FOUR_THREE, 1.5).scale(), 1.5 * NARROW_SCALE_FLOOR));
    for shape in [SIXTEEN_NINE, TWENTY_ONE_NINE, THIRTY_TWO_NINE] {
        assert!(near(HudBox::new(shape, 1.25).scale(), 1.25), "no shrink at 16:9 and wider");
    }
}

#[test]
fn a_window_with_no_size_or_a_bad_scale_falls_back_to_the_canvas_at_scale_1() {
    let empty = Screen { width: 0.0, height: 0.0, pixels_per_point: 1.0 };
    assert_eq!(HudBox::new(empty, 1.0), HudBox::new(CANVAS, 1.0));
    assert!(near(HudBox::new(SIXTEEN_NINE, f32::NAN).scale(), 1.0));
    assert!(near(HudBox::new(SIXTEEN_NINE, -2.0).scale(), 1.0));
    assert_eq!(HudBox::default(), HudBox::new(CANVAS, 1.0));
}

#[test]
fn the_centre_anchor_is_the_canvas_centre() {
    let frame = HudBox::new(TWENTY_ONE_NINE, 1.0);
    assert_eq!(frame.point(Anchor::Centre), (0.0, 0.0));
    let map = frame.place(Anchor::Centre, (0.0, 0.0), (200.0, 120.0));
    assert!(near(map.x, 0.0) && near(map.y, 0.0));
}
