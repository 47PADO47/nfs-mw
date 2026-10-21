//! The gauntlet rewards: what the last race of a rival's gauntlet hands out, and the popup queue that holds the
//! rewards of a race that runs under the "deferred" attribute (`docs/specs/story-flow-2.md` section 4.5 and F-25;
//! `docs/specs/career-scripts-events.md` section 3; `docs/specs/career-economy-flows.md` section 6.5;
//! `docs/formats/career-save.md`, "Marker and unlock inventory").
//!
//! Pure logic. The caller reads the reward collection of the race (the rewards are data it owns), reads the race's
//! "deferred" attribute (hash `0xF2FE50D7`, F-25) and passes both in. Nothing here shows a screen or opens a file.
//!
//! Routing, one reward at a time (F-25, economy-flows 6.5):
//! - a reward with a part (1 to 16) is a part unlock. Its part number is the entry type in the marker inventory, the
//!   same table the markers use, and the effect [`RewardEffect::PartUnlocked`] reports it. A part takes precedence
//!   over a kind.
//! - kind `0x12` is a pink slip: the rival's preset car (`BL<n>`) goes to the stable. The stable has no add call that
//!   needs only a name, so this module reports [`RewardEffect::AddCar`] and the game crate resolves the preset to its
//!   catalogue car and writes the record. It is not stored in the inventory.
//! - kind `0x13` is a cash bonus: `value` is added to the profile cash at once. It is not stored in the inventory.
//! - any other non-zero kind (`0x11` Get Out of Jail Free, `0x14` Extra Impound Strike, `0x15` Impound Release) goes
//!   into the first free inventory slot with no duplicate check.
//!
//! Deferred rewards: when the race carries the "deferred" attribute, every routable reward goes to the popup queue
//! ([`PopupQueue`], six slots, not saved) instead of being applied. The safe house markers screen reads that queue
//! later and calls [`grant`] with `deferred` false. Routing is checked first, so a reward that has no route is never
//! queued.
//!
//! The caps and the one-time rules, and what the specs leave open (**unconfirmed**):
//! - Inventory full: the marker is not stored and [`RewardEffect::InventoryFull`] reports it. The original's
//!   behaviour when all 63 slots are taken is not seen **[unconfirmed]**.
//! - Popup queue full: the reward is not queued and [`RewardEffect::QueueFull`] returns it. Not seen **[unconfirmed]**.
//! - Pink slip repeats: the caller says which presets are already in the stable (`owns_preset`). A preset named twice
//!   in one list is granted once. Whether the original checks the stable or a flag is not seen **[unconfirmed]**.
//! - Gauntlet rewards are granted once, on the first win of the last race (F-10). The game crate calls
//!   [`grant_gauntlet_rewards`] after the final boss race is won. Nothing in
//!   [`super::apply_event_result`] calls it, because that function does not know which race ends a gauntlet or what
//!   its reward list holds.

use crate::economy::markers::MarkerInventory;
use crate::settings::CareerSettings;

/// The kind of a pink slip: the rival's preset car is added to the stable (F-25).
pub const KIND_PINK_SLIP: u8 = 0x12;
/// The kind of a cash bonus: `value` is added to the profile cash (F-25).
pub const KIND_CASH_BONUS: u8 = 0x13;
/// The first part-unlock number (brakes).
pub const PART_FIRST: u32 = 1;
/// The last part-unlock number (paint).
pub const PART_LAST: u32 = 0x10;
/// The number of popup queue slots (`career-economy-flows.md` 6.5: six 12-byte slots).
pub const POPUP_SLOTS: usize = 6;

/// One reward of a race, as the game crate reads it from the reward collection. Each field is what the collection
/// holds: `part` is the upgrade type, `kind` the marker kind, `value` the cash amount (kind `0x13`) and `preset` the
/// preset car name (kind `0x12`, format `BL%d`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reward {
    /// The part-unlock number, 1 to 16. `None` when the reward is not a part.
    pub part: Option<u32>,
    /// The marker kind. `None` when the reward has no kind.
    pub kind: Option<u8>,
    /// The cash amount of a cash bonus. Not read for other kinds. A negative value has no route.
    pub value: i32,
    /// The preset car name of a pink slip, such as `BL05`. Not read for other kinds.
    pub preset: Option<String>,
}

/// What one reward did. [`grant`] returns one per reward, in the order of the list, so nothing is dropped silently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewardEffect {
    /// Cash was added to the profile (kind `0x13`). Carries the amount added.
    CashAdded(u32),
    /// A marker was stored in the inventory. Carries the kind (`0x11`, `0x14`, `0x15` or another non-zero kind).
    MarkerAdded { kind: u8 },
    /// A part was unlocked. The part is stored in the inventory with the part number as its type.
    PartUnlocked { part: u32 },
    /// The preset car must be added to the stable. The game crate resolves `preset` to its catalogue car and writes
    /// the record. The car is not in the stable yet and nothing else has been written for it.
    AddCar { preset: String },
    /// The preset car is already in the stable (`owns_preset`), or an earlier reward of the same list granted it.
    /// Nothing is added.
    CarAlreadyOwned { preset: String },
    /// The reward went to the popup queue because the race is deferred.
    Queued(Reward),
    /// The popup queue had no free slot. The reward was not queued and is returned here.
    QueueFull(Reward),
    /// The marker inventory had no free slot. A marker or part was not stored. The reward is returned here.
    InventoryFull(Reward),
    /// The reward has no route: no part and no kind, a part outside 1 to 16, a kind of 0 (the free-slot type), a
    /// pink slip with no preset, or a cash bonus with a negative value. Nothing is written and nothing is queued.
    Unknown(Reward),
}

/// The popup queue: the rewards of deferred races, waiting for the safe house markers screen. Six slots, in the
/// order they were queued. The queue is not saved (`career-economy-flows.md` 6.5).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PopupQueue {
    entries: Vec<Reward>,
}

impl PopupQueue {
    /// An empty queue.
    pub fn new() -> PopupQueue {
        PopupQueue::default()
    }

    /// The queued rewards, in queue order.
    pub fn entries(&self) -> &[Reward] {
        &self.entries
    }

    /// True when all [`POPUP_SLOTS`] slots are taken.
    pub fn is_full(&self) -> bool {
        self.entries.len() >= POPUP_SLOTS
    }

    /// Queues `reward` at the end. Returns it back as the error when the queue is full.
    pub fn push(&mut self, reward: Reward) -> Result<(), Reward> {
        if self.is_full() {
            return Err(reward);
        }
        self.entries.push(reward);
        Ok(())
    }

    /// Takes the reward at `index` out of the queue. **[unconfirmed]** the order of the rest is kept; the original's
    /// removal is not seen. Returns `None` when `index` is outside the queue.
    pub fn take(&mut self, index: usize) -> Option<Reward> {
        if index >= self.entries.len() {
            return None;
        }
        Some(self.entries.remove(index))
    }
}

/// Where a reward goes once it has a route.
enum Route<'a> {
    Part(u32),
    Car(&'a str),
    Cash(u32),
    Marker(u8),
}

/// The route of `reward`, or `None` when it has none (see [`RewardEffect::Unknown`]).
fn route(reward: &Reward) -> Option<Route<'_>> {
    if let Some(part) = reward.part {
        return (PART_FIRST..=PART_LAST).contains(&part).then_some(Route::Part(part));
    }
    let kind = reward.kind?;
    match kind {
        0 => None,
        KIND_PINK_SLIP => reward.preset.as_deref().map(Route::Car),
        KIND_CASH_BONUS => u32::try_from(reward.value).ok().map(Route::Cash),
        other => Some(Route::Marker(other)),
    }
}

/// Grants `rewards` to the career, in order. With `deferred` every routable reward is queued in `popups` instead of
/// being applied. `owns_preset` says whether a preset car is already in the stable; a preset is also skipped when an
/// earlier reward of this list granted it. Returns one effect per reward.
///
/// The caller decides `deferred` from the race's "deferred" attribute and whether the list is granted at all (see
/// [`grant_gauntlet_rewards`] for the gauntlet's one-time rule).
pub fn grant(
    rewards: &[Reward],
    deferred: bool,
    career: &mut CareerSettings,
    popups: &mut PopupQueue,
    owns_preset: impl Fn(&str) -> bool,
) -> Vec<RewardEffect> {
    let mut granted_cars: Vec<&str> = Vec::new();
    rewards.iter().map(|reward| grant_one(reward, deferred, career, popups, &owns_preset, &mut granted_cars)).collect()
}

/// Grants the reward list of a gauntlet's last race. The game crate calls this after the final boss race of the
/// gauntlet is won: [`super::apply_event_result`] does not, because it knows neither the gauntlet's last race nor
/// its reward list. `first_win` is whether this is the first win of that race; the rewards are granted only then
/// (F-10, CB 4.2 step 5). A repeat win returns no effects and changes nothing.
pub fn grant_gauntlet_rewards(
    rewards: &[Reward],
    first_win: bool,
    deferred: bool,
    career: &mut CareerSettings,
    popups: &mut PopupQueue,
    owns_preset: impl Fn(&str) -> bool,
) -> Vec<RewardEffect> {
    if !first_win {
        return Vec::new();
    }
    grant(rewards, deferred, career, popups, owns_preset)
}

fn grant_one<'a>(
    reward: &'a Reward,
    deferred: bool,
    career: &mut CareerSettings,
    popups: &mut PopupQueue,
    owns_preset: &impl Fn(&str) -> bool,
    granted_cars: &mut Vec<&'a str>,
) -> RewardEffect {
    let Some(target) = route(reward) else {
        return RewardEffect::Unknown(reward.clone());
    };
    if deferred {
        return match popups.push(reward.clone()) {
            Ok(()) => RewardEffect::Queued(reward.clone()),
            Err(rejected) => RewardEffect::QueueFull(rejected),
        };
    }
    match target {
        Route::Cash(amount) => {
            career.add_cash(amount);
            RewardEffect::CashAdded(amount)
        }
        Route::Part(part) => match store_entry(career, part) {
            true => RewardEffect::PartUnlocked { part },
            false => RewardEffect::InventoryFull(reward.clone()),
        },
        Route::Marker(kind) => match store_entry(career, u32::from(kind)) {
            true => RewardEffect::MarkerAdded { kind },
            false => RewardEffect::InventoryFull(reward.clone()),
        },
        Route::Car(preset) => grant_car(preset, owns_preset, granted_cars),
    }
}

/// A pink slip: the car is reported for the game crate to add, unless the stable already has it or this list did.
fn grant_car<'a>(
    preset: &'a str,
    owns_preset: &impl Fn(&str) -> bool,
    granted_cars: &mut Vec<&'a str>,
) -> RewardEffect {
    if granted_cars.contains(&preset) || owns_preset(preset) {
        return RewardEffect::CarAlreadyOwned { preset: preset.to_owned() };
    }
    granted_cars.push(preset);
    RewardEffect::AddCar { preset: preset.to_owned() }
}

/// Stores one entry `{kind, aux 0, flag 1}` in the first free inventory slot. Returns false when the table is full.
fn store_entry(career: &mut CareerSettings, kind: u32) -> bool {
    let Ok(mut inventory) = MarkerInventory::from_table(&career.inventory[..]) else {
        return false;
    };
    if inventory.add_entries(kind, 0, 1) == 0 {
        return false;
    }
    career.inventory.copy_from_slice(&inventory.to_table());
    true
}

#[cfg(test)]
#[path = "rewards_tests.rs"]
mod tests;
