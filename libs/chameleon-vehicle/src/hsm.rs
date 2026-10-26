//! The state machines (`HSMData`, resource type 0x501) of a 2012 car bundle, as far as the animation states go.
//! Spec: `docs/formats/chameleon-active-aero.md`.
//!
//! A machine is a block of objects. An object is a record of eight words (type hash, id, three table offsets, a zero, the
//! name and a flags word), a **child list** (16 bytes per child: id, table, count, name) and a **property table** of
//! 16-byte rows: a 64-bit key, the offset of a value cell, a type and a flag. A value cell holds the offset of a text; a
//! row of type 6 holds a table offset and a row count instead, which is a nested object. Read here: the animation state
//! (its animation, and the condition that closes it), the conditions (a number, the input it is tested against, an operator)
//! and the machine's child lists, which put a condition before the state it opens.

/// The type hashes of the records.
const MACHINE: u32 = 0x24c4_22b2;
const STATE: u32 = 0x849c_8ce8;
const CONDITION: u32 = 0x0ec6_f1b1;
const TRANSITION: u32 = 0x809a_cb7d;
/// The keys of a state's properties: the animation (a decimal number) and the condition that closes it.
const KEY_ANIMATION: u64 = 0x4f88_3ba0_b304_9400;
const KEY_CLOSE: u64 = 0x6845_63f7_fd6c_4600;
/// The keys of a condition's data: the number it is compared with, the input's name and the operator.
const KEY_VALUE: u64 = 0xbcda_a6b6_34a9_aa00;
const KEY_INPUT: u64 = 0x9952_fc58_1900_0000;
const KEY_OPERATOR: u64 = 0x5b55_835f_4e5a_7080;
/// The key of the seconds of a timed transition (`Wait for 5s`).
const KEY_TIMER: u64 = 0xbd69_b4a6_1900_0000;
/// The row of a condition that is 1 when any of its tests is enough.
const KEY_ANY: u64 = 0x5b56_2d0c_8c7b_8000;
/// The row type of a nested object.
const OBJECT: u16 = 6;
/// Range of the ids of the objects of a machine.
const IDS: std::ops::Range<u32> = 0x1000..0x0100_0000;

/// A condition's data: an input compared with a number.
#[derive(Debug, Clone, PartialEq)]
pub struct Threshold {
    pub input: String,
    /// The operator code as text: `0` equal, `1` not equal, `2` greater than, `3` less than, `4` at most, `5` at least
    /// (read off the labels of the conditions: `Brake OFF` is `brake` `0` `0`, `Brake ON` is `1`, `Turning Left` is
    /// `steering` below -0.3 and `Turning Right` above 0.3).
    pub operator: String,
    pub value: f32,
}

/// An animation state of a machine, with the conditions that open and close it where the machine has them in the plain form.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub name: String,
    /// The id of the animation (without the namespace byte).
    pub animation: u64,
    /// The tests that open the state, all of which have to hold: the condition listed just before it in its machine.
    pub open: Vec<Threshold>,
    /// The tests kept in the state's own properties, that close it. Empty: none.
    pub close: Vec<Threshold>,
    /// Whether the state has children of its own (conditions and timed transitions to other states).
    pub has_children: bool,
}

struct Record {
    kind: u32,
    id: u32,
    children: usize,
    props: usize,
    name: usize,
}

struct Row {
    key: u64,
    cell: usize,
    kind: u16,
    /// The flag word: how many objects an object row holds.
    count: usize,
}

fn word(block: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(block.get(at..at + 4)?.try_into().ok()?))
}

fn half(block: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(block.get(at..at + 2)?.try_into().ok()?))
}

/// The NUL-terminated text at `at`, when it is printable ASCII (it may be empty).
fn text_at(block: &[u8], at: usize) -> Option<String> {
    let tail = block.get(at..)?;
    let end = tail.iter().position(|&b| b == 0)?;
    let text = &tail[..end];
    (end < 64 && text.iter().all(|b| (32..127).contains(b))).then(|| String::from_utf8_lossy(text).into_owned())
}

/// The record that starts at `at`, if one does: a known type hash, an id in range and a name that reads.
fn record_at(block: &[u8], at: usize) -> Option<Record> {
    let kind = word(block, at)?;
    if ![MACHINE, STATE, CONDITION, TRANSITION].contains(&kind) {
        return None;
    }
    let id = word(block, at + 4)?;
    let (name, children, props) =
        (word(block, at + 24)? as usize, word(block, at + 12)? as usize, word(block, at + 16)? as usize);
    let valid = IDS.contains(&id) && name != 0 && text_at(block, name).is_some() && props < block.len();
    valid.then_some(Record { kind, id, children, props, name })
}

fn records(block: &[u8]) -> Vec<Record> {
    (0..block.len().saturating_sub(32)).step_by(4).filter_map(|at| record_at(block, at)).collect()
}

/// The property rows of the table at `at`: rows until one does not look like a row.
fn rows(block: &[u8], at: usize) -> Vec<Row> {
    let mut out = Vec::new();
    let mut at = at;
    while at != 0 && at + 16 <= block.len() {
        let (Some(low), Some(high), Some(cell), Some(kind), Some(flag)) =
            (word(block, at), word(block, at + 4), word(block, at + 8), half(block, at + 12), half(block, at + 14))
        else {
            break;
        };
        let key = u64::from(low) | (u64::from(high) << 32);
        let valid = key & 0x3f == 0
            && key >> 8 != 0
            && (1..block.len()).contains(&(cell as usize))
            && flag <= 8
            && kind <= OBJECT;
        if !valid {
            break;
        }
        out.push(Row { key, cell: cell as usize, kind, count: usize::from(flag) });
        at += 16;
    }
    out
}

/// The text a row holds.
fn text_of(block: &[u8], row: &Row) -> Option<String> {
    (row.kind != OBJECT).then(|| text_at(block, word(block, row.cell)? as usize)).flatten()
}

/// The nested objects a row holds: its flag word counts them (a condition that is several tests at once has one per test),
/// and its cell holds a table offset and a row count for each.
fn objects_of(block: &[u8], row: &Row) -> Vec<Vec<Row>> {
    if row.kind != OBJECT {
        return Vec::new();
    }
    (0..row.count.max(1))
        .filter_map(|n| {
            let at = row.cell + 8 * n;
            let (table, count) = (word(block, at)? as usize, word(block, at + 4)? as usize);
            let mut found = rows(block, table);
            found.truncate(count);
            (found.len() == count).then_some(found)
        })
        .collect()
}

/// A condition's data in the rows of an object.
fn threshold(block: &[u8], object: &[Row]) -> Option<Threshold> {
    let text = |key: u64| object.iter().find(|row| row.key == key).and_then(|row| text_of(block, row));
    let value = text(KEY_VALUE)?.parse::<f32>().ok().filter(|value| value.is_finite())?;
    Some(Threshold { input: text(KEY_INPUT)?, operator: text(KEY_OPERATOR)?, value })
}

/// The tests in the nested objects of a row: all of them have to hold.
fn thresholds_of(block: &[u8], row: &Row) -> Vec<Threshold> {
    objects_of(block, row).iter().filter_map(|object| threshold(block, object)).collect()
}

/// The condition of a condition record: the tests among its properties.
fn condition_of(block: &[u8], record: &Record) -> Vec<Threshold> {
    rows(block, record.props)
        .iter()
        .map(|row| thresholds_of(block, row))
        .find(|found| !found.is_empty())
        .unwrap_or_default()
}

/// The ids in the child list at `at`: entries of 16 bytes while the first word is the id of a record.
fn children(block: &[u8], at: usize, known: &[Record]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut at = at;
    while at != 0 {
        let Some(id) = word(block, at).filter(|id| known.iter().any(|record| record.id == *id)) else { break };
        out.push(id);
        at += 16;
    }
    out
}

/// An air vent: an animation state with the delays of the timed transitions around it.
#[derive(Debug, Clone, PartialEq)]
pub struct Vent {
    pub name: String,
    pub animation: u64,
    /// The speed it opens above, and the seconds the speed has to stay above it.
    pub open_mph: f32,
    pub open_seconds: f32,
    /// The speed it closes below, and the seconds it waits after that before it closes.
    pub close_mph: f32,
    pub close_seconds: f32,
}

/// The seconds of a timed transition record.
fn timer_of(block: &[u8], record: &Record) -> Option<f32> {
    let row = rows(block, record.props).into_iter().find(|row| row.key == KEY_TIMER)?;
    text_of(block, &row)?.parse::<f32>().ok().filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
}

/// The speed test (input `speedinmph`, operator `op`) that a record holds as its only condition, or that its single child
/// condition holds (a timed transition names the test that aborts its wait).
fn speed_test(block: &[u8], all: &[Record], record: &Record, op: &str) -> Option<f32> {
    let own = condition_of(block, record);
    let tests = match own.is_empty() {
        true => children(block, record.children, all)
            .iter()
            .filter_map(|id| all.iter().find(|candidate| candidate.id == *id && candidate.kind == CONDITION))
            .flat_map(|condition| condition_of(block, condition))
            .collect(),
        false => own,
    };
    match &tests[..] {
        [test] if test.input == "speedinmph" && test.operator == op => Some(test.value),
        _ => None,
    }
}

/// The air vents of the machine in `block`: animation states that have children of their own (a condition that closes them
/// and a timed transition), opened by the nearest condition listed before them, with a timed transition listed in the machine whose
/// aborting condition is the opposite of that one.
pub fn vents(block: &[u8]) -> Vec<Vent> {
    let all = records(block);
    let mut out = Vec::new();
    for state in all.iter().filter(|record| record.kind == STATE) {
        let props = rows(block, state.props);
        let Some(animation) = props
            .iter()
            .find(|row| row.key == KEY_ANIMATION)
            .and_then(|row| text_of(block, row))
            .and_then(|text| text.parse::<u64>().ok())
        else {
            continue;
        };
        let own: Vec<&Record> = children(block, state.children, &all)
            .iter()
            .filter_map(|id| all.iter().find(|candidate| candidate.id == *id))
            .collect();
        let close = own.iter().find_map(|record| match record.kind {
            CONDITION => speed_test(block, &all, record, "3"),
            _ => None,
        });
        let close_wait = own.iter().find(|record| record.kind == TRANSITION).and_then(|record| timer_of(block, record));
        let machine = all.iter().filter(|record| record.kind == MACHINE).find_map(|record| {
            let list = children(block, record.children, &all);
            list.iter().position(|id| *id == state.id).map(|at| (list, at))
        });
        let Some((list, at)) = machine else { continue };
        let open = list[..at]
            .iter()
            .rev()
            .find_map(|id| all.iter().find(|candidate| candidate.id == *id && candidate.kind == CONDITION))
            .and_then(|record| speed_test(block, &all, record, "2"));
        let open_wait = list
            .iter()
            .filter_map(|id| all.iter().find(|candidate| candidate.id == *id && candidate.kind == TRANSITION))
            .find(|record| open.is_some() && speed_test(block, &all, record, "3") == open)
            .and_then(|record| timer_of(block, record));
        let (Some(close_mph), Some(close_seconds), Some(open_mph), Some(open_seconds)) =
            (close, close_wait, open, open_wait)
        else {
            continue;
        };
        out.push(Vent {
            name: text_at(block, state.name).unwrap_or_default(),
            animation,
            open_mph,
            open_seconds,
            close_mph,
            close_seconds,
        });
    }
    out
}

/// A condition record: its tests, and whether any of them is enough (otherwise all have to hold).
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionRecord {
    pub name: String,
    pub tests: Vec<Threshold>,
    pub any: bool,
}

/// The conditions of the machine in `block` with the way their tests combine (the row `5b562d0c8c7b8000` is 1 for `OR`).
pub fn condition_records(block: &[u8]) -> Vec<ConditionRecord> {
    records(block)
        .iter()
        .filter(|record| record.kind == CONDITION)
        .map(|record| ConditionRecord {
            name: text_at(block, record.name).unwrap_or_default(),
            tests: condition_of(block, record),
            any: rows(block, record.props)
                .iter()
                .find(|row| row.key == KEY_ANY)
                .and_then(|row| text_of(block, row))
                .is_some_and(|text| text == "1"),
        })
        .collect()
}

/// The seconds of every timed transition of the machine in `block`.
pub fn timers(block: &[u8]) -> Vec<f32> {
    let all = records(block);
    all.iter().filter(|record| record.kind == TRANSITION).filter_map(|record| timer_of(block, record)).collect()
}

/// The data of every condition of the machine in `block`.
pub fn conditions(block: &[u8]) -> Vec<Threshold> {
    records(block)
        .iter()
        .filter(|record| record.kind == CONDITION)
        .flat_map(|record| condition_of(block, record))
        .collect()
}

/// The animation states of the machine in `block`.
pub fn states(block: &[u8]) -> Vec<State> {
    let all = records(block);
    let lists: Vec<(u32, Vec<u32>)> = all
        .iter()
        .filter(|record| record.kind == MACHINE)
        .map(|record| (record.id, children(block, record.children, &all)))
        .collect();
    let mut out = Vec::new();
    for record in all.iter().filter(|record| record.kind == STATE) {
        let props = rows(block, record.props);
        let prop = |key: u64| props.iter().find(|row| row.key == key);
        let Some(animation) =
            prop(KEY_ANIMATION).and_then(|row| text_of(block, row)).and_then(|text| text.parse::<u64>().ok())
        else {
            continue;
        };
        let close = prop(KEY_CLOSE).map(|row| thresholds_of(block, row)).unwrap_or_default();
        let open = lists
            .iter()
            .find_map(|(_, list)| {
                let at = list.iter().position(|&id| id == record.id)?;
                let before = list.get(at.checked_sub(1)?)?;
                let condition = all.iter().find(|candidate| candidate.id == *before && candidate.kind == CONDITION)?;
                Some(condition_of(block, condition))
            })
            .unwrap_or_default();
        out.push(State {
            name: text_at(block, record.name).unwrap_or_default(),
            animation,
            open,
            close,
            has_children: !children(block, record.children, &all).is_empty(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A block with a machine that lists a condition and a state, as the 911's does.
    fn machine(open: (&str, &str, &str), close: Option<(&str, &str, &str)>, waits: bool) -> Vec<u8> {
        let mut b = vec![0u8; 0x400];
        let put = |b: &mut Vec<u8>, at: usize, v: u32| b[at..at + 4].copy_from_slice(&v.to_le_bytes());
        let text = |b: &mut Vec<u8>, at: usize, s: &str| b[at..at + s.len()].copy_from_slice(s.as_bytes());
        // Names at 0x300..: machine, condition, state, then the strings of the values.
        text(&mut b, 0x300, "Machine");
        text(&mut b, 0x310, "Above");
        text(&mut b, 0x320, "Anim");
        // A data object (3 rows) at `at` for the strings `(value, input, operator)`, strings stored from `s`.
        let object = |b: &mut Vec<u8>, at: usize, s: usize, v: (&str, &str, &str)| {
            let keys = [KEY_VALUE, KEY_INPUT, KEY_OPERATOR];
            let strings = [v.0, v.1, v.2];
            for k in 0..3 {
                let row = at + 16 * k;
                let cell = at + 48 + 4 * k;
                let at_text = s + 16 * k;
                text(b, at_text, strings[k]);
                put(b, row, keys[k] as u32);
                put(b, row + 4, (keys[k] >> 32) as u32);
                put(b, row + 8, cell as u32);
                put(b, row + 12, 0x0001_0005);
                put(b, cell, at_text as u32);
            }
        };
        // Records at 0x30 (machine), 0x50 (condition), 0x70 (state).
        let record = |b: &mut Vec<u8>, at: usize, kind: u32, id: u32, children: u32, props: u32, name: u32| {
            for (k, v) in [kind, id, 0, children, props, 0, name, 0x0001_0102].into_iter().enumerate() {
                put(b, at + 4 * k, v);
            }
        };
        record(&mut b, 0x30, MACHINE, 0x1000_00, 0x200, 0, 0x300);
        record(&mut b, 0x50, CONDITION, 0x1000_02, 0, 0x100, 0x310);
        record(&mut b, 0x70, STATE, 0x1000_04, if waits { 0x230 } else { 0 }, 0x170, 0x320);
        // The machine's children: the condition, then the state.
        for (k, id) in [0x1000_02u32, 0x1000_04].into_iter().enumerate() {
            put(&mut b, 0x200 + 16 * k, id);
        }
        if waits {
            put(&mut b, 0x230, 0x1000_02);
        }
        // The condition's property table: one object row at 0x100, its object at 0x120 (3 rows), strings at 0x340.
        put(&mut b, 0x100, 0xff05_55c0);
        put(&mut b, 0x104, 0x5b56_2b76);
        put(&mut b, 0x108, 0x110);
        put(&mut b, 0x10c, 0x0001_0006);
        put(&mut b, 0x110, 0x120);
        put(&mut b, 0x114, 3);
        object(&mut b, 0x120, 0x340, open);
        // The state's table at 0x170: the animation, and a close object at 0x1a0 when there is one.
        put(&mut b, 0x170, KEY_ANIMATION as u32);
        put(&mut b, 0x174, (KEY_ANIMATION >> 32) as u32);
        put(&mut b, 0x178, 0x190);
        put(&mut b, 0x17c, 0x0001_0005);
        put(&mut b, 0x190, 0x194);
        text(&mut b, 0x194, "904078");
        if let Some(close) = close {
            put(&mut b, 0x180, KEY_CLOSE as u32);
            put(&mut b, 0x184, (KEY_CLOSE >> 32) as u32);
            put(&mut b, 0x188, 0x1a0);
            put(&mut b, 0x18c, 0x0001_0006);
            put(&mut b, 0x1a0, 0x1c0);
            put(&mut b, 0x1a4, 3);
            object(&mut b, 0x1c0, 0x370, close);
        }
        b
    }

    #[test]
    fn a_state_has_the_condition_before_it_to_open_and_its_own_to_close() {
        let block = machine(("70", "speedinmph", "2"), Some(("40", "speedinmph", "3")), false);
        let found = states(&block);
        assert_eq!(found.len(), 1);
        let state = &found[0];
        assert_eq!((state.animation, state.name.as_str(), state.has_children), (904078, "Anim", false));
        assert_eq!(state.open, vec![Threshold { input: "speedinmph".into(), operator: "2".into(), value: 70.0 }]);
        assert_eq!(state.close, vec![Threshold { input: "speedinmph".into(), operator: "3".into(), value: 40.0 }]);
    }

    #[test]
    fn a_state_with_children_and_one_without_a_close_condition_are_told_apart() {
        let waits = states(&machine(("75", "speedinmph", "2"), None, true));
        assert!(waits[0].has_children && waits[0].close.is_empty());
        assert!(states(&[0u8; 64]).is_empty(), "a block with no record has no state");
    }
}
