//! The reward lists of races: the reward collections a race grants on its first completion (attribute `0x50104D90`,
//! a list of references to collections of the `gameplay` class). Spec: `docs/specs/story-flow-2.md` section 4.5 and
//! F-25; `docs/specs/career-blacklist.md` section 6 step 5; `docs/specs/career-economy-flows.md` section 5 (marker
//! kinds). Field names are not known for most of these attributes, so they are read by their keys.

use blackbox_attrib::{CollectionRef, Database, Value};

use super::{collection, key_array_at, name_of};

/// The reward list of a race: references to reward collections, in order.
const REWARD_LIST_FIELD: u32 = 0x5010_4D90;
/// The upgrade type of a reward (`0x0E0113FE`): a non-zero value unlocks that shop part (`story-flow-2.md` F-25).
const PART_FIELD: u32 = 0x0E01_13FE;
/// The first marker-kind text of a reward (`0x06A077D5`); the kind is named by it together with the second text.
const KIND_TEXT_FIELD: u32 = 0x06A0_77D5;
/// The second marker-kind text of a reward (`0x3C2FDAAB`).
const KIND_TEXT2_FIELD: u32 = 0x3C2F_DAAB;
/// The value of a reward (`0xAB0179F4`): the cash a cash bonus (kind `0x13`) adds to the profile.
const VALUE_FIELD: u32 = 0xAB01_79F4;

/// One reward a race grants on its first completion.
#[derive(Debug, Clone, PartialEq)]
pub struct RewardRecord {
    /// The reward collection's key (the hash of its name).
    pub key: u32,
    /// The reward collection's name, for example `race_bin_16/16_2_3_sprint/reward_out_of_jail_free1`.
    pub name: String,
    /// The upgrade type field (`0x0E0113FE`). No reward of the install sets it (census), so it is `None` throughout;
    /// the parts are named by the kind texts instead (see [`RewardRecord::kind`]).
    pub part: Option<u32>,
    /// The first marker-kind text (`0x06A077D5`), trimmed; `None` when empty or unset.
    pub kind_text: Option<String>,
    /// The second marker-kind text (`0x3C2FDAAB`), trimmed; `None` when empty or unset.
    pub kind_text2: Option<String>,
    /// The reward's value (`0xAB0179F4`): the cash of a cash bonus. 0 when unset.
    pub value: i32,
}

impl RewardRecord {
    /// The marker kind the texts name: `0x11` out of jail free, `0x12` pink slip, `0x13` cash bonus, `0x14` extra
    /// impound strike and `0x15` impound release (`docs/specs/career-economy-flows.md` section 5). A `backroom`
    /// category names a part by its item text, kind 1 to 16 in the order of `docs/specs/story-flow-2.md` section 4.5.
    /// `None` for any other text.
    pub fn kind(&self) -> Option<u8> {
        let category = self.kind_text.as_deref();
        if category == Some("backroom") {
            return self.kind_text2.as_deref().and_then(part_kind);
        }
        [category, self.kind_text2.as_deref()].into_iter().flatten().find_map(kind_of_text)
    }
}

/// The marker kind of one of the documented kind texts.
fn kind_of_text(text: &str) -> Option<u8> {
    Some(match text {
        "out_of_jail_free" => 0x11,
        "pink_slip" => 0x12,
        "cash_bonus" => 0x13,
        "add_impound_box" => 0x14,
        "release_car_from_impound" => 0x15,
        _ => return None,
    })
}

/// The part kind of a `backroom` item: 1 brakes, 2 engine, 3 nitrous, 4 induction, 5 chassis, 6 tires, 7
/// transmission, 8 body kit, 9 hood, 10 spoiler, 11 rims, 12 roof scoop, 13 HUD, 14 vinyls, 15 decals, 16 paint.
fn part_kind(item: &str) -> Option<u8> {
    const PARTS: [&str; 16] = [
        "brakes",
        "engine",
        "nitrous",
        "induction",
        "chassis",
        "tires",
        "transmission",
        "bodykit",
        "hood",
        "spoiler",
        "rims",
        "roofscoop",
        "hud",
        "vinyls",
        "decals",
        "paint",
    ];
    PARTS.iter().position(|part| *part == item).map(|index| index as u8 + 1)
}

/// The reward list of a race, resolved in order. A key that names no collection is skipped.
pub(super) fn read_list(db: &Database, race: CollectionRef<'_>) -> Vec<RewardRecord> {
    key_array_at(race, REWARD_LIST_FIELD).into_iter().filter_map(|key| collection(db, key)).map(record).collect()
}

fn record(c: CollectionRef<'_>) -> RewardRecord {
    RewardRecord {
        key: c.key(),
        name: name_of(c),
        part: c.get_by_key(PART_FIELD).and_then(Value::as_u32).filter(|part| *part != 0),
        kind_text: text_at(c, KIND_TEXT_FIELD),
        kind_text2: text_at(c, KIND_TEXT2_FIELD),
        value: c.get_by_key(VALUE_FIELD).and_then(Value::as_i32).unwrap_or(0),
    }
}

/// A text field read by its key, trimmed; `None` when unset or empty.
fn text_at(c: CollectionRef<'_>, field_key: u32) -> Option<String> {
    c.get_by_key(field_key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reward(kind_text: Option<&str>, kind_text2: Option<&str>) -> RewardRecord {
        RewardRecord {
            key: 1,
            name: "reward".to_owned(),
            part: None,
            kind_text: kind_text.map(str::to_owned),
            kind_text2: kind_text2.map(str::to_owned),
            value: 0,
        }
    }

    #[test]
    fn the_documented_texts_give_their_marker_kinds() {
        assert_eq!(reward(Some("out_of_jail_free"), None).kind(), Some(0x11));
        assert_eq!(reward(None, Some("cash_bonus")).kind(), Some(0x13));
        assert_eq!(reward(Some("add_impound_box"), None).kind(), Some(0x14));
        assert_eq!(reward(Some("release_car_from_impound"), None).kind(), Some(0x15));
    }

    #[test]
    fn the_pink_slip_and_the_backroom_parts_give_their_kinds() {
        assert_eq!(reward(Some("pink_slip"), None).kind(), Some(0x12));
        assert_eq!(reward(Some("backroom"), Some("brakes")).kind(), Some(1));
        assert_eq!(reward(Some("backroom"), Some("roofscoop")).kind(), Some(12));
        assert_eq!(reward(Some("backroom"), Some("paint")).kind(), Some(16));
        assert_eq!(reward(Some("backroom"), Some("unknown_part")).kind(), None);
    }

    #[test]
    fn an_undocumented_text_has_no_kind() {
        assert_eq!(reward(Some("other_text"), Some("other")).kind(), None);
        assert_eq!(reward(None, None).kind(), None);
    }
}
