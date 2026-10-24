//! Tests for the Custom menus' geometry: the places the components draw at and the frontend's hit tests read.

use glam::Vec2;

use super::*;

const WIDE: f32 = 16.0 / 9.0;

#[test]
fn the_list_starts_at_thirty_percent_of_the_box_inside_the_safe_area() {
    let frame = MenuFrame::new(WIDE, 1.0);
    assert!((frame.list_top() - (-240.0 + 0.3 * 480.0)).abs() < 1e-3);
    assert!(frame.plate_left() > frame.safe().min_x(), "the plates start inside the safe area");
    assert!(frame.text_left() > frame.plate_left());
    assert!(frame.title_y() < frame.list_top(), "the title sits above the list");
    assert!(frame.footer_y() > frame.row_y(MAX_ROWS - 1), "the footer sits under the last row");
    assert!(frame.footer_y() < frame.safe().max_y());
}

#[test]
fn eight_rows_fit_above_the_footer_at_every_text_size() {
    for text_size in [0.8, 1.0, 1.25, 1.5] {
        for aspect in [4.0 / 3.0, WIDE, 21.0 / 9.0, 32.0 / 9.0] {
            let frame = MenuFrame::new(aspect, text_size);
            let last = frame.row_rect(frame.row_y(MAX_ROWS - 1), true);
            assert!(last.max_y() < frame.footer_y() - 4.0, "{text_size} at {aspect}: the last row clears the footer");
            assert!(frame.row_right(true) <= frame.safe().max_x() + 1e-3, "a settings row stays inside the safe area");
        }
    }
    let small = MenuFrame::new(WIDE, 0.8);
    let large = MenuFrame::new(WIDE, 1.2);
    assert!(large.pitch() > small.pitch(), "a larger text size spreads the rows");
}

#[test]
fn the_menus_keep_to_the_central_box_on_an_ultrawide_window() {
    let wide = MenuFrame::new(WIDE, 1.0);
    let ultra = MenuFrame::new(32.0 / 9.0, 1.0);
    assert!((wide.plate_left() - ultra.plate_left()).abs() < 1e-3, "the list keeps to the 16:9 box");
    assert!(ultra.window().w > wide.window().w);
}

#[test]
fn a_row_answers_across_its_band_up_to_the_end_of_its_plate() {
    let frame = MenuFrame::new(WIDE, 1.0);
    let left = frame.text_left();
    assert_eq!(frame.row_at(Vec2::new(left, frame.row_y(3)), 8, false), Some(3));
    assert_eq!(frame.row_at(Vec2::new(left, frame.row_y(0) + frame.pitch() * 0.49), 8, false), Some(0));
    let past = Vec2::new(frame.row_right(false) + 1.0, frame.row_y(3));
    assert_eq!(frame.row_at(past, 8, false), None, "right of an entry's plate");
    assert_eq!(frame.row_at(past, 8, true), Some(3), "a settings row's plate runs on");
    assert_eq!(frame.row_at(Vec2::new(left, frame.row_y(4)), 3, false), None, "only the shown rows answer");
    assert_eq!(frame.row_at(Vec2::new(left, frame.list_top() - 5.0), 8, false), None, "above the list");
}

#[test]
fn a_click_on_a_settings_row_lands_on_a_chevron_or_the_middle() {
    let frame = MenuFrame::new(WIDE, 1.0);
    let column = frame.value_column(0.0);
    assert_eq!(frame.side_at(frame.text_left()), Side::Middle, "on the label");
    assert_eq!(frame.side_at(column.min_x() + 3.0), Side::Left);
    assert_eq!(frame.side_at(column.max_x() - 3.0), Side::Right);
    assert_eq!(frame.side_at(column.x), Side::Middle, "on the value");
    assert_eq!(frame.along_value(column.min_x() - 10.0), 0.0);
    assert_eq!(frame.along_value(column.max_x() + 10.0), 1.0);
    assert!((frame.along_value(column.x) - 0.5).abs() < 1e-4);
    assert!(frame.label_room(true) > 150.0, "a label has room before the values");
    assert!(frame.label_room(false) > frame.label_room(true));
}

#[test]
fn a_window_with_no_shape_and_a_bad_text_size_fall_back() {
    let frame = MenuFrame::new(f32::NAN, f32::INFINITY);
    assert_eq!(frame, MenuFrame::new(4.0 / 3.0, 1.0));
    assert_eq!((shown(3), shown(8), shown(20)), (3, 8, MAX_ROWS));
}
