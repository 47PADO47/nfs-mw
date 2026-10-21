//! The safe house markers screen (`SafeHouseMarkers.fng`, `docs/specs/story-flow-2.md` F-24, `docs/specs/career-safehouse.md`
//! 5.4, `docs/specs/career-text.md` 7.1). Step 1 of the arrival sequence after a rival is beaten.
//!
//! The rewards of the rival's gauntlet wait in the popup queue ([`PopupQueue`]). The screen turns the queue into cards:
//! the queued rewards of the four card categories, in category order, with the first three shuffled. The player reveals
//! two cards. A revealed card is applied at once ([`grant_rewards`]: a pink slip adds the car to the stable, cash is
//! added, any other marker goes into the inventory). Continue empties the queue and steps the arrival sequence (the
//! screen's exit is a `Command::Resume`, which the sequence turns into its next step, `flow_arrival.rs`).
//!
//! The category table is native and was not read. The order used here, which the spec leaves open:
//! 1. parts (back-room part unlocks, 1 to 16),
//! 2. out of jail and impound (Get Out of Jail Free `0x11`, Extra Impound Strike `0x14`, Impound Release `0x15`),
//! 3. the pink slip (`0x12`, which needs a preset car),
//! 4. the cash bonus (`0x13`, which needs a value of 0 or more).
//!
//! A reward with no category (any other kind, no route) is not offered. The shuffle swaps each of the first three cards
//! with a random one of the first three (F-24) and draws from the quick play generator.
//!
//! Object names of the package are not known yet (the dump needs a build), so the FEng look binds only the Accept
//! button: Left and Right move the highlight over the cards and Continue, Accept presses the highlighted row. The Remastered
//! look draws the cards as a list ([`ScreenLogic::list`]), which shows the card names once they are revealed. The
//! pure parts (categories, the cards, the two-pick deck) are below and tested in `markers_tests.rs`.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_LEFT, PAD_RIGHT};
use blackbox_text::label_hash;
use nfsmw_career::blacklist::{KIND_CASH_BONUS, KIND_PINK_SLIP, PART_FIRST, PART_LAST, Reward};

use crate::career::{CareerHandle, grant_rewards};
use crate::frontend::ids::screen;
use crate::frontend::logic::{Args, Command, Cx, Pick, ScreenLogic};
use crate::frontend::quick_play::pool::Rng;
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuStyle};
use crate::ui::{Text, UiAssets};

/// The kind of Get Out of Jail Free (`0x11`, `career-text.md` 7.1).
const KIND_JAIL_FREE: u8 = 0x11;
/// The kind of an extra impound strike (`0x14`).
const KIND_EXTRA_STRIKE: u8 = 0x14;
/// The kind of an impound release (`0x15`).
const KIND_IMPOUND_RELEASE: u8 = 0x15;

/// The most cards the player reveals (F-24: "may reveal two").
pub const PICKS: usize = 2;
/// The number of leading cards that are shuffled (F-24: "the first three").
pub const SHUFFLED: usize = 3;

/// The label key of each part unlock, by part number minus one (`career-text.md` 7.1). The induction's label is
/// `TURBO` here; the table lists `TURBO` and `SUPERCHARGER` for it. The last two have their own keys.
const PART_KEYS: [&str; 16] = [
    "MARKER_NAME_BRAKES",
    "MARKER_NAME_ENGINE",
    "MARKER_NAME_NOS",
    "MARKER_NAME_TURBO",
    "MARKER_NAME_CHASSI",
    "MARKER_NAME_TIRE",
    "MARKER_NAME_TRANSMISSION",
    "MARKER_NAME_BODYKIT",
    "MARKER_NAME_HOOD",
    "MARKER_NAME_SPOILER",
    "MARKER_NAME_RIMS",
    "MARKER_NAME_ROOFSCOOPS",
    "MARKER_NAME_HUD",
    "MARKER_NAME_VINYLS",
    "CB_MARKERS_DECAL",
    "CB_MARKERS_PAINT",
];

/// The four card categories, in the order the cards are listed (the declaration order is the sort order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    /// A back-room part unlock (parts 1 to 16).
    Parts,
    /// Get Out of Jail Free, Extra Impound Strike or Impound Release.
    Escape,
    /// A pink slip: the rival's car.
    PinkSlip,
    /// A cash bonus.
    Cash,
}

/// The category a queued reward is offered under, or `None` when it is not offered. A part outside 1 to 16 and a pink slip
/// with no preset have no route ([`grant_rewards`] would skip them), so they are not offered either.
pub fn category_of(reward: &Reward) -> Option<Category> {
    if let Some(part) = reward.part {
        return (PART_FIRST..=PART_LAST).contains(&part).then_some(Category::Parts);
    }
    match reward.kind? {
        KIND_JAIL_FREE | KIND_EXTRA_STRIKE | KIND_IMPOUND_RELEASE => Some(Category::Escape),
        KIND_PINK_SLIP => reward.preset.is_some().then_some(Category::PinkSlip),
        KIND_CASH_BONUS => u32::try_from(reward.value).is_ok().then_some(Category::Cash),
        _ => None,
    }
}

/// One card of the screen: the reward it stands for, and whether the player has revealed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub reward: Reward,
    pub revealed: bool,
}

/// The cards of a queue: the offered rewards in category order (the queue's order within a category), then the first
/// [`SHUFFLED`] of them shuffled. Every card starts unrevealed.
pub fn build_cards(queue: &[Reward], rng: &mut Rng) -> Vec<Card> {
    let mut offered: Vec<(Category, Reward)> =
        queue.iter().filter_map(|reward| Some((category_of(reward)?, reward.clone()))).collect();
    // The sort is stable, so the queue's order is kept within each category.
    offered.sort_by_key(|(category, _)| *category);
    let mut rewards: Vec<Reward> = offered.into_iter().map(|(_, reward)| reward).collect();
    shuffle_head(&mut rewards, rng);
    rewards.into_iter().map(|reward| Card { reward, revealed: false }).collect()
}

/// Swaps each of the first [`SHUFFLED`] items with a random one of them (F-24). Items past the head do not move.
fn shuffle_head<T>(items: &mut [T], rng: &mut Rng) {
    let head = items.len().min(SHUFFLED);
    for i in 0..head {
        let j = rng.below(head);
        items.swap(i, j);
    }
}

/// The cards and the picks made on them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Deck {
    cards: Vec<Card>,
    picks: usize,
}

impl Deck {
    /// A deck of these cards, with no picks made.
    pub fn new(cards: Vec<Card>) -> Deck {
        Deck { cards, picks: 0 }
    }

    /// The cards, in the order they are shown.
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    /// True while the player may still reveal a card: fewer than [`PICKS`] made, and fewer than the cards there are.
    pub fn can_pick(&self) -> bool {
        self.picks < PICKS.min(self.cards.len())
    }

    /// Continue is offered once every pick the cards allow is made (two, or fewer when fewer cards are offered).
    pub fn can_continue(&self) -> bool {
        !self.can_pick()
    }

    /// Reveals the card at `index` and returns its reward, which the caller applies. `None` when the player has no pick
    /// left, the index is outside the cards, or the card is already revealed. Only the picks count: a card that is not
    /// picked is lost when the screen leaves.
    pub fn reveal(&mut self, index: usize) -> Option<Reward> {
        if !self.can_pick() {
            return None;
        }
        let card = self.cards.get_mut(index)?;
        if card.revealed {
            return None;
        }
        card.revealed = true;
        self.picks += 1;
        Some(card.reward.clone())
    }
}

/// The label key of a reward's marker (`career-text.md` 7.1), or `None` when the reward has no label.
fn label_key(reward: &Reward) -> Option<&'static str> {
    if let Some(part) = reward.part {
        let index = usize::try_from(part.checked_sub(PART_FIRST)?).ok()?;
        return PART_KEYS.get(index).copied();
    }
    match reward.kind? {
        KIND_JAIL_FREE => Some("MARKER_NAME_FREEBAIL"),
        KIND_EXTRA_STRIKE => Some("MARKER_NAME_IMPOUND"),
        KIND_IMPOUND_RELEASE => Some("MARKER_NAME_RELEASE"),
        KIND_PINK_SLIP => Some("MARKER_NAME_PINKSLIP"),
        KIND_CASH_BONUS => Some("MARKER_NAME_CASH"),
        _ => None,
    }
}

/// The name of a card's marker from the game's text, or a plain word when the table has none.
fn card_name(assets: &UiAssets, reward: &Reward) -> String {
    label_key(reward)
        .and_then(|key| Text::Game(label_hash(key)).resolve(assets))
        .unwrap_or_else(|| "Marker".to_string())
}

/// The row after the cards: Continue.
const CONTINUE_LABEL: &str = "Continue";

/// Makes the screen for `name` (lower case) when it is the safe house markers screen.
pub fn make(name: &str, _args: Args) -> Option<Box<dyn ScreenLogic>> {
    if name != screen::SAFEHOUSE_MARKERS.to_ascii_lowercase() {
        return None;
    }
    Some(Box::new(MarkersLogic::new(Rng::from_clock())))
}

/// The markers screen: the cards of the queue, the highlight, and the picks.
pub struct MarkersLogic {
    rng: Rng,
    deck: Deck,
    /// The highlighted row: a card, or the Continue row after the last card.
    cursor: usize,
    /// Whether the Remastered look draws the cards as a list (`Cx::vertical_look`).
    vertical: bool,
}

impl MarkersLogic {
    /// A screen whose shuffle draws from `rng`. The cards are made when the screen starts.
    pub fn new(rng: Rng) -> Self {
        Self { rng, deck: Deck::default(), cursor: 0, vertical: false }
    }

    /// Reveals the card at `index` if the player may pick it, and applies its reward at once.
    fn reveal(&mut self, index: usize) {
        let Some(reward) = self.deck.reveal(index) else { return };
        let Some(effects) = CareerHandle::shared().with(|career| grant_rewards(career, &[reward], false)) else {
            return log::warn!("markers: no career is played, so the revealed reward is not granted");
        };
        log::info!("markers: card {index} revealed: {effects:?}");
    }

    /// Presses a row: a card reveals it, the Continue row leaves once the picks are made.
    fn press(&mut self, cx: &mut Cx, row: usize) {
        if row < self.deck.cards().len() {
            return self.reveal(row);
        }
        if self.deck.can_continue() {
            leave(cx);
        }
    }

    fn move_cursor(&mut self, down: bool) {
        let last = self.deck.cards().len();
        self.cursor = if down { (self.cursor + 1).min(last) } else { self.cursor.saturating_sub(1) };
    }
}

/// Continue: the queue is emptied, the rewards not picked are lost, and the sequence goes on.
fn leave(cx: &mut Cx) {
    CareerHandle::shared().with(|career| career.popups.clear());
    cx.send(Command::Resume);
}

impl ScreenLogic for MarkersLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.vertical = cx.vertical_look();
        let queue = CareerHandle::shared().with(|career| career.popups.entries().to_vec()).unwrap_or_default();
        self.deck = Deck::new(build_cards(&queue, &mut self.rng));
        log::info!("markers: {} card(s) offered from {} queued reward(s)", self.deck.cards().len(), queue.len());
        // Nothing to choose: the queue is emptied and the sequence goes on at once (as the region screen does).
        if self.deck.cards().is_empty() {
            leave(cx);
        }
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.vertical = cx.vertical_look();
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT => self.move_cursor(false),
            PAD_RIGHT => self.move_cursor(true),
            PAD_ACCEPT => {
                let row = self.cursor;
                self.press(cx, row);
            }
            _ => {}
        }
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        if row > self.deck.cards().len() {
            return;
        }
        self.cursor = row;
        if matches!(pick, Pick::Press(..)) {
            self.press(cx, row);
        }
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    /// Up and Down move the highlight as Left and Right do (the Remastered look's list).
    fn rotate_pad(&self) -> bool {
        self.vertical
    }

    fn list(&self, assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let mut entries: Vec<MenuEntry> = self
            .deck
            .cards()
            .iter()
            .enumerate()
            .map(|(index, card)| {
                let label =
                    if card.revealed { card_name(assets, &card.reward) } else { format!("Marker {}", index + 1) };
                MenuEntry::plain(label, !card.revealed && self.deck.can_pick())
            })
            .collect();
        entries.push(MenuEntry::plain(CONTINUE_LABEL, self.deck.can_continue()));
        Some(MenuList {
            title: "Markers".into(),
            message: String::new(),
            entries,
            selected: self.cursor,
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Sub,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }
}

#[cfg(test)]
#[path = "markers_tests.rs"]
mod tests;
