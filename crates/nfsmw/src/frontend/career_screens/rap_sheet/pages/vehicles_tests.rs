use super::*;

#[test]
fn each_status_has_its_own_label() {
    assert_eq!(status_key(DriveStatus::Impounded), "RAP_VD_IMPOUNDED");
    assert_eq!(status_key(DriveStatus::ConfirmedToDrive), "RAP_VD_CONFIRMED_TO_DRIVE");
    assert_eq!(status_key(DriveStatus::SuspectedToDrive), "RAP_VD_SUSPECTED_TO_DRIVE");
}

#[test]
fn the_list_moves_one_row_and_stops_at_the_last_full_page() {
    assert_eq!(scroll(0, 5, Step::Down), 1);
    assert_eq!(scroll(1, 5, Step::Down), 2);
    assert_eq!(scroll(2, 5, Step::Down), 3);
    assert_eq!(scroll(3, 5, Step::Down), 3, "five cars, two on the page: the last top is 3");
    assert_eq!(scroll(0, 5, Step::Up), 0);
    assert_eq!(scroll(3, 5, Step::Up), 2);
}

#[test]
fn a_short_list_never_scrolls() {
    assert_eq!(scroll(0, 0, Step::Down), 0);
    assert_eq!(scroll(0, 1, Step::Down), 0);
    assert_eq!(scroll(0, 2, Step::Down), 0);
}

#[test]
fn a_header_line_shows_the_car_name_as_it_is() {
    assert_eq!(car_text(Some("PORSCHE 911 TURBO")), "PORSCHE 911 TURBO");
}
