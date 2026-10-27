use super::*;

fn designs(count: usize) -> Vec<PlateDesignChoice> {
    (1..=count).map(|n| PlateDesignChoice { id: n.to_string(), name: format!("DESIGN {n}"), group: None }).collect()
}

fn page(count: usize, at: usize) -> PlateDesigns {
    PlateDesigns {
        handle: GarageUi::default(),
        folder: "CAR".into(),
        entries: entries(&designs(count)),
        at,
        before: None,
        vertical: false,
        done: false,
    }
}

#[test]
fn the_plain_plate_comes_first_and_the_designs_follow_in_order() {
    let list = entries(&designs(3));
    let ids: Vec<&str> = list.iter().map(|entry| entry.id.as_str()).collect();
    assert_eq!(ids, ["", "1", "2", "3"]);
    assert_eq!(list[0].label, "Plain plate");
    assert_eq!(list[2].label, "DESIGN 2");
    assert_eq!(entries(&[]).len(), 1, "a car with no designs still has the plain plate");
}

#[test]
fn the_cursor_starts_on_the_design_worn_or_on_the_plain_plate() {
    let list = entries(&designs(3));
    assert_eq!(position(&list, Some("2")), 2);
    assert_eq!(position(&list, Some("")), 0, "the plain plate, picked on purpose");
    assert_eq!(position(&list, None), 0, "no design worn");
    assert_eq!(position(&list, Some("gone")), 0, "a design the index no longer lists");
}

#[test]
fn stepping_wraps_round_at_both_ends() {
    assert_eq!(stepped(0, -1, 5), 4);
    assert_eq!(stepped(4, 1, 5), 0);
    assert_eq!(stepped(2, 10, 5), 2);
    assert_eq!(stepped(1, -10, 5), 1);
    assert_eq!(stepped(0, 1, 470), 1);
    assert_eq!(stepped(0, 1, 0), 0, "an empty list never moves");
}

#[test]
fn the_window_follows_the_cursor_and_stays_inside_the_list() {
    assert_eq!(window_start(0, 470), 0);
    assert_eq!(window_start(100, 470), 96, "the cursor in the middle of nine names");
    assert_eq!(window_start(469, 470), 461, "the last nine");
    assert_eq!(window_start(2, 5), 0, "a short list is shown whole");
}

#[test]
fn the_notice_says_which_design_of_how_many() {
    assert_eq!(page(3, 0).shown(), "Design 1 of 4: Plain plate");
    assert_eq!(page(3, 2).shown(), "Design 3 of 4: DESIGN 2");
}
