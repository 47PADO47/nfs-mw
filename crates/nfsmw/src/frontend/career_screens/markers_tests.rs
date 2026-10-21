use super::*;

/// A reward of the kind, with a value for cash and a preset for a pink slip.
fn kind(kind: u8) -> Reward {
    Reward { kind: Some(kind), ..Reward::default() }
}

fn part(number: u32) -> Reward {
    Reward { part: Some(number), ..Reward::default() }
}

fn pink_slip(preset: &str) -> Reward {
    Reward { kind: Some(KIND_PINK_SLIP), preset: Some(preset.to_string()), ..Reward::default() }
}

fn cash(value: i32) -> Reward {
    Reward { kind: Some(KIND_CASH_BONUS), value, ..Reward::default() }
}

fn cards_of(queue: &[Reward], seed: u64) -> Vec<Card> {
    build_cards(queue, &mut Rng::new(seed))
}

#[test]
fn the_four_categories_take_their_parts_and_kinds() {
    assert_eq!(category_of(&part(PART_FIRST)), Some(Category::Parts));
    assert_eq!(category_of(&part(PART_LAST)), Some(Category::Parts));
    assert_eq!(category_of(&kind(0x11)), Some(Category::Escape), "Get Out of Jail Free");
    assert_eq!(category_of(&kind(0x14)), Some(Category::Escape), "Extra Impound Strike");
    assert_eq!(category_of(&kind(0x15)), Some(Category::Escape), "Impound Release");
    assert_eq!(category_of(&pink_slip("BL05")), Some(Category::PinkSlip));
    assert_eq!(category_of(&cash(0)), Some(Category::Cash));
    assert_eq!(category_of(&cash(500)), Some(Category::Cash));
}

#[test]
fn a_reward_with_no_route_has_no_category_and_is_not_offered() {
    assert_eq!(category_of(&part(0)), None, "part 0 is not a part");
    assert_eq!(category_of(&part(PART_LAST + 1)), None, "part 17 is not a part");
    assert_eq!(category_of(&Reward::default()), None, "no part and no kind");
    assert_eq!(category_of(&kind(0)), None, "kind 0 is the free slot");
    assert_eq!(category_of(&kind(0x16)), None, "an unknown kind");
    assert_eq!(
        category_of(&Reward { kind: Some(KIND_PINK_SLIP), ..Reward::default() }),
        None,
        "a pink slip with no preset"
    );
    assert_eq!(category_of(&cash(-1)), None, "a negative cash bonus");
}

#[test]
fn a_part_takes_precedence_over_a_kind_as_it_does_when_granted() {
    let both = Reward { part: Some(3), kind: Some(KIND_PINK_SLIP), preset: Some("BL05".into()), ..Reward::default() };
    assert_eq!(category_of(&both), Some(Category::Parts));
}

#[test]
fn the_cards_are_in_category_order_and_only_the_first_three_are_shuffled() {
    let queue = [cash(10), pink_slip("BL05"), kind(0x11), part(2), cash(20), cash(30)];
    for seed in 0..40 {
        let cards = cards_of(&queue, seed);
        assert_eq!(cards.len(), 6, "every routable reward is a card");
        let head: Vec<Category> = cards[..3].iter().map(|card| category_of(&card.reward).unwrap()).collect();
        let mut sorted_head = head.clone();
        sorted_head.sort();
        assert_eq!(sorted_head, [Category::Parts, Category::Escape, Category::PinkSlip], "seed {seed}");
        // Past the head the order is unchanged: the three cash bonuses, in the queue's order.
        assert_eq!(cards[3].reward.value, 10, "seed {seed}");
        assert_eq!(cards[4].reward.value, 20, "seed {seed}");
        assert_eq!(cards[5].reward.value, 30, "seed {seed}");
    }
}

#[test]
fn the_head_of_the_cards_is_shuffled_for_some_seed() {
    let queue = [part(1), kind(0x11), pink_slip("BL05")];
    let unshuffled: Vec<Reward> = vec![part(1), kind(0x11), pink_slip("BL05")];
    let moved = (0..40).any(|seed| {
        let cards = cards_of(&queue, seed);
        cards.iter().map(|card| card.reward.clone()).collect::<Vec<_>>() != unshuffled
    });
    assert!(moved, "the swap moves the cards for some seed");
}

#[test]
fn the_same_seed_gives_the_same_cards() {
    let queue = [part(1), kind(0x11), pink_slip("BL05"), cash(5)];
    assert_eq!(cards_of(&queue, 7), cards_of(&queue, 7));
}

#[test]
fn an_unroutable_queue_entry_is_not_a_card() {
    let queue = [part(17), kind(0x16), cash(1)];
    let cards = cards_of(&queue, 1);
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].reward, cash(1));
    assert!(cards.iter().all(|card| !card.revealed), "every card starts unrevealed");
}

#[test]
fn an_empty_queue_has_no_cards() {
    assert!(cards_of(&[], 3).is_empty());
}

#[test]
fn the_player_reveals_two_cards_and_no_more() {
    let mut deck = Deck::new(cards_of(&[part(1), kind(0x11), cash(1), cash(2), cash(3)], 2));
    assert!(deck.can_pick());
    assert!(!deck.can_continue(), "Continue waits for the picks");
    assert!(deck.reveal(0).is_some());
    assert!(deck.reveal(0).is_none(), "a revealed card cannot be picked again");
    assert!(deck.reveal(1).is_some());
    assert!(!deck.can_pick());
    assert!(deck.can_continue());
    assert!(deck.reveal(2).is_none(), "the second pick used both picks");
    assert_eq!(deck.cards().iter().filter(|card| card.revealed).count(), 2);
}

#[test]
fn a_revealed_card_gives_the_reward_it_stands_for() {
    let mut deck = Deck::new(cards_of(&[pink_slip("BL05"), cash(40)], 0));
    let first = deck.cards()[0].reward.clone();
    assert_eq!(deck.reveal(0), Some(first));
    assert!(deck.reveal(9).is_none(), "an index past the cards is no pick");
}

#[test]
fn with_one_card_one_pick_is_the_whole_choice() {
    let mut deck = Deck::new(cards_of(&[kind(0x15)], 0));
    assert!(!deck.can_continue());
    assert_eq!(deck.reveal(0), Some(kind(0x15)));
    assert!(deck.can_continue());
}

#[test]
fn with_no_cards_continue_is_offered_at_once() {
    let mut deck = Deck::new(Vec::new());
    assert!(!deck.can_pick());
    assert!(deck.can_continue());
    assert!(deck.reveal(0).is_none());
}

#[test]
fn each_kind_has_its_label_key() {
    assert_eq!(label_key(&part(1)), Some("MARKER_NAME_BRAKES"));
    assert_eq!(label_key(&part(15)), Some("CB_MARKERS_DECAL"));
    assert_eq!(label_key(&part(16)), Some("CB_MARKERS_PAINT"));
    assert_eq!(label_key(&part(17)), None);
    assert_eq!(label_key(&kind(0x11)), Some("MARKER_NAME_FREEBAIL"));
    assert_eq!(label_key(&pink_slip("BL05")), Some("MARKER_NAME_PINKSLIP"));
    assert_eq!(label_key(&cash(1)), Some("MARKER_NAME_CASH"));
    assert_eq!(label_key(&kind(0x16)), None);
}

#[test]
fn make_takes_only_the_markers_screen() {
    let own = screen::SAFEHOUSE_MARKERS.to_ascii_lowercase();
    assert!(make(&own, Args::default()).is_some());
    assert!(make("safehouseregionunlock.fng", Args::default()).is_none());
}
