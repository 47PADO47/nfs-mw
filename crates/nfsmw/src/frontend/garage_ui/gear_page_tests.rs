use super::*;

fn list(count: usize) -> Vec<Entry> {
    std::iter::once(Entry { id: String::new(), label: "Stock".into() })
        .chain((1..=count).map(|n| Entry { id: format!("ID{n}"), label: format!("ITEM {n}") }))
        .collect()
}

fn page(part: Part, count: usize, at: usize) -> GearPage {
    GearPage { handle: GarageUi::default(), part, entries: list(count), at, vertical: false, done: true }
}

#[test]
fn the_cursor_starts_on_the_item_worn_or_on_the_riders_own_gear() {
    let entries = list(3);
    assert_eq!(position(&entries, "ID2"), 2);
    assert_eq!(position(&entries, ""), 0, "his own gear");
    assert_eq!(position(&entries, "gone"), 0, "an item the catalogue no longer lists");
}

#[test]
fn the_notice_says_which_item_of_how_many_for_the_part() {
    assert_eq!(page(Part::Helmet, 3, 0).shown(), "Rider Helmet 1 of 4: Stock");
    assert_eq!(page(Part::Gloves, 3, 2).shown(), "Rider Gloves 3 of 4: ITEM 2");
    assert_eq!(page(Part::Body, 1, 1).shown(), "Rider Body 2 of 2: ITEM 1");
}

#[test]
fn a_page_whose_part_has_nothing_offered_is_the_riders_own_gear_alone() {
    let alone = [Entry { id: String::new(), label: "Stock".into() }];
    assert_eq!(position(&alone, "anything"), 0);
    assert_eq!(stepped(0, 1, alone.len()), 0, "there is nowhere to step to");
}
