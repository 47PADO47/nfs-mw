//! The career Blacklist bins: one `race_bin` collection per rank, in the `gameplay` class. Spec:
//! `docs/specs/career-blacklist.md` sections 1 and 8. Only the fields the spec lists are read. Fields it marks as
//! unconfirmed are not read.

use blackbox_attrib::{CollectionRef, Database, Value};

use super::{inherits, instances, key_array, key_field, key_of, name_of};

/// The parent template of every bin collection.
const BIN_TEMPLATE: &str = "race_bin";
/// Field `0xBAF89280` (`GCollectionKey[]`): the `unlocks/upgrade_*` collections granted on entering the bin
/// (spec section 5.2). The field name is not known, so the key is read by its hash.
const UNLOCKS_FIELD: u32 = 0xBAF8_9280;

/// One Blacklist bin: the rival it leads to, the three counters that open the rival challenge and the bin's races.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bin {
    /// The collection's key (the hash of its name).
    pub key: u32,
    /// For example `race_bin_10`.
    pub name: String,
    /// `BinIndex`: 1..16 and 18..21 (16 is the prologue, 18 to 21 are the side bins).
    pub number: i32,
    /// `RequiredRacesWon`: races of `world_races` to win before the rival challenge opens.
    pub required_races_won: i32,
    /// `RequiredChallenges`: milestones and speed traps of the bin to pass.
    pub required_challenges: i32,
    /// `RequiredBounty`: total bounty needed.
    pub required_bounty: i32,
    /// `Boss`: the rival's `boss` collection key.
    pub boss: Option<u32>,
    /// `BossRaces`: the rival's races, in order (what the challenge button starts).
    pub boss_races: Vec<u32>,
    /// `WorldRaces`: the free races of the bin, in order (counted for races won).
    pub world_races: Vec<u32>,
    /// `Children`: every race, trigger and unlock collection under the bin, in order.
    pub children: Vec<u32>,
    /// `EntryCellCallID`: the phone call queued when the bin starts. `None` when 0 or unset.
    pub entry_call: Option<u32>,
    /// `SMSRivalChallenge`: the message sent when the rival unlocks. `None` when -1 (none).
    pub rival_sms: Option<u32>,
    /// `SMSCellChallenge`: the message sent when the last challenge is passed. `None` when -1 (none).
    pub cell_challenge_sms: Option<u32>,
    /// `CellChallengeRace`: the race unlocked when the last challenge is passed (bin 15 only).
    pub cell_challenge_race: Option<u32>,
    /// `EntryActivity`: the activity run on entry (bin 16 only).
    pub entry_activity: Option<u32>,
    /// `Barriers`: the scenery groups (`BARRIER_LOCKED_n`) switched on while this bin is current.
    pub barriers: Vec<String>,
    /// The unlock list (field `0xBAF89280`): the `unlocks/upgrade_*` collections granted on entering the bin.
    pub unlocks: Vec<u32>,
}

/// Every Blacklist bin, sorted by bin number. The database order is not documented, so the order is fixed here.
pub fn bins(db: &Database) -> Vec<Bin> {
    let mut bins: Vec<Bin> = instances(db).filter(|c| inherits(*c, BIN_TEMPLATE)).filter_map(bin).collect();
    sort_by_number(&mut bins);
    bins
}

fn bin(c: CollectionRef<'_>) -> Option<Bin> {
    // A bin without its number is not a Blacklist bin.
    let number = c.get_i32("BinIndex")?;
    Some(Bin {
        key: c.key(),
        name: name_of(c),
        number,
        required_races_won: c.get_i32("RequiredRacesWon").unwrap_or(0),
        required_challenges: c.get_i32("RequiredChallenges").unwrap_or(0),
        required_bounty: c.get_i32("RequiredBounty").unwrap_or(0),
        boss: key_field(c, "Boss"),
        boss_races: key_array(c, "BossRaces"),
        world_races: key_array(c, "WorldRaces"),
        children: key_array(c, "Children"),
        entry_call: message_id(c.get_i32("EntryCellCallID")).filter(|id| *id != 0),
        rival_sms: message_id(c.get_i32("SMSRivalChallenge")),
        cell_challenge_sms: message_id(c.get_i32("SMSCellChallenge")),
        cell_challenge_race: key_field(c, "CellChallengeRace"),
        entry_activity: key_field(c, "EntryActivity"),
        barriers: texts_in(c.get("Barriers")),
        unlocks: keys_in(c.get_by_key(UNLOCKS_FIELD)),
    })
}

/// A message id field. Negative values mean "no message" (the spec's -1), so they give `None`.
fn message_id(raw: Option<i32>) -> Option<u32> {
    raw.and_then(|id| u32::try_from(id).ok())
}

/// A `GCollectionKey[]` value, in order, without the empty entries. A single key is taken as a one-item list.
fn keys_in(value: Option<&Value>) -> Vec<u32> {
    match value {
        Some(Value::Array(items)) => items.iter().filter_map(key_of).filter(|k| *k != 0).collect(),
        Some(value) => key_of(value).filter(|k| *k != 0).into_iter().collect(),
        None => Vec::new(),
    }
}

/// A `text[]` value, trimmed, in order, without the empty entries. Anything that is not an array gives no entries.
fn texts_in(value: Option<&Value>) -> Vec<String> {
    let items = match value {
        Some(Value::Array(items)) => items,
        _ => return Vec::new(),
    };
    items
        .iter()
        .filter_map(|v| match v {
            Value::Text(Some(s)) => Some(s.trim()),
            _ => None,
        })
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Sorts by bin number, then by key, so equal numbers keep a fixed order.
fn sort_by_number(bins: &mut [Bin]) {
    bins.sort_by_key(|b| (b.number, b.key));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_ids_drop_the_negative_none_value() {
        assert_eq!(message_id(Some(-1)), None);
        assert_eq!(message_id(Some(0)), Some(0));
        assert_eq!(message_id(Some(89)), Some(89));
        assert_eq!(message_id(None), None);
    }

    #[test]
    fn entry_call_zero_means_no_call() {
        // The entry call uses the zero-is-none rule on top of the message id.
        assert_eq!(message_id(Some(0)).filter(|id| *id != 0), None);
        assert_eq!(message_id(Some(125)).filter(|id| *id != 0), Some(125));
    }

    #[test]
    fn key_arrays_keep_order_and_drop_empty_keys() {
        let items = Value::Array(vec![
            Value::Key(7),
            Value::Key(0),
            Value::Raw { type_key: 0x6CD4_B70A, bytes: vec![21, 81, 19, 151] },
            Value::Key(9),
        ]);
        assert_eq!(keys_in(Some(&items)), vec![7, 0x9713_5115, 9]);
    }

    #[test]
    fn a_single_key_is_a_one_item_list() {
        assert_eq!(keys_in(Some(&Value::Key(3))), vec![3]);
        assert_eq!(keys_in(Some(&Value::Key(0))), Vec::<u32>::new());
        assert_eq!(keys_in(None), Vec::<u32>::new());
    }

    #[test]
    fn text_arrays_are_trimmed_and_skip_empty_entries() {
        let items = Value::Array(vec![
            Value::Text(Some(" BARRIER_LOCKED_10 ".to_owned())),
            Value::Text(None),
            Value::Text(Some("  ".to_owned())),
            Value::Text(Some("BARRIER_LOCKED_12".to_owned())),
        ]);
        assert_eq!(texts_in(Some(&items)), vec!["BARRIER_LOCKED_10", "BARRIER_LOCKED_12"]);
        assert_eq!(texts_in(Some(&Value::Key(1))), Vec::<String>::new());
        assert_eq!(texts_in(None), Vec::<String>::new());
    }

    #[test]
    fn bins_sort_by_number_then_key() {
        let mut bins = vec![
            Bin { key: 9, number: 18, ..Default::default() },
            Bin { key: 4, number: 1, ..Default::default() },
            Bin { key: 2, number: 3, ..Default::default() },
            Bin { key: 1, number: 1, ..Default::default() },
        ];
        sort_by_number(&mut bins);
        let order: Vec<(i32, u32)> = bins.iter().map(|b| (b.number, b.key)).collect();
        assert_eq!(order, vec![(1, 1), (1, 4), (3, 2), (18, 9)]);
    }
}
