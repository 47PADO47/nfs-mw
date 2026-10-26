//! The active spoiler of a 2012 car. Spec: `docs/formats/chameleon-active-aero.md`.
//!
//! Some cars raise a spoiler when they are fast. The bundle holds a state machine (`HSMData`, resource type 0x501) named
//! `<car>_SpoilerAnimation`. Its conditions test the car's inputs (`speedinmph`, `gas`, `brake`, `steering`: the names the
//! executable binds to the vehicle) against numbers, and its state plays an animation (type
//! 0xB3) of the skeleton. The animation state behaviour of the executable opens when the input goes above
//! the open threshold and closes when it falls below the close threshold, advancing the animation clock forwards or
//! backwards at real time between its first key and its end.
//!
//! Only the machines that are just that are read here: two conditions on `speedinmph` (one `>`, one `<`) and one
//! animation. A machine with more states, or with inputs other than the speed, returns `None`.

use chameleon_bundle::{Bundle, ResourceKind};

use crate::skeleton::Skeleton;

/// The resource type of a state machine.
const HSM_KIND: u32 = 0x501;
/// The keys of the properties of an `HSM.AnimationCondition` object: the number it compares with, the input's name and the
/// operator. Their meaning is read off the values they hold (the input names, and the condition labels).
const KEY_VALUE: u64 = 0xbcda_a6b6_34a9_aa00;
const KEY_INPUT: u64 = 0x9952_fc58_1900_0000;
const KEY_OPERATOR: u64 = 0x5b55_835f_4e5a_7080;
/// The operator codes of a condition: 2 is "greater than" and 3 is "less than". Every speed condition of the install that
/// the condition labels name (`Speed above N`, `Speed under N`) has these codes.
const GREATER: &str = "2";
const LESS: &str = "3";
/// The input the spoiler machines test.
const SPEED_INPUT: &str = "speedinmph";

/// Offsets in an animation resource: its length in seconds, its format and reference key, and the counts and tables.
const DURATION_AT: usize = 0x30;
const FORMAT_AT: usize = 0x34;
const REFERENCE_AT: usize = 0x36;
const KEYS_AT: usize = 0x04;
const BONES_AT: usize = 0x08;
const ROTATION_STRIDE_AT: usize = 0x1c;
const TRANSLATION_STRIDE_AT: usize = 0x20;
/// Where the offsets of the key tables and the bone maps are (a word each from here: 1 the translations, 4 their bone map,
/// 5 the packed rotations, 7 their bone map).
const TABLES_AT: usize = 0x38;
/// The id of a resource without its namespace byte: what the state machine's strings call an animation.
const ID_MASK: u64 = 0x00FF_FFFF_FFFF;
/// Smallest movement and turn taken for one: metres and degrees.
const MIN_TRAVEL: f32 = 1e-3;
const MIN_TURN_DEG: f32 = 0.05;
/// What a packed rotation key's three 10-bit fields are made of: `(field - 511) * SCALE`
/// (the executable's constants; see the spec).
const PACK_CENTRE: f32 = 511.0;
const PACK_SCALE: f32 = f32::from_bits(0x3ab5_3240);
/// The animation format that packs a rotation into 32 bits.
const PACKED_FORMAT: u16 = 1;

/// What a spoiler does: where it moves to, how long it takes and at which speeds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpoilerMotion {
    /// The bone that moves: the skeleton's `SpoilerHinge`.
    pub bone: usize,
    /// The movement of the bone between the animation's first key and its last, metres, in the engine's space (x left,
    /// y up, z forward); the bone is turned first and then moved.
    pub translation: [f32; 3],
    /// The turn between the same keys as a unit quaternion (x, y, z, w) in the engine's space; identity for none.
    pub rotation: [f32; 4],
    /// Length of the animation, seconds: the time the whole movement takes, either way.
    pub seconds: f32,
    /// The animation opens when the speed is above this, mph.
    pub above_mph: f32,
    /// It closes when the speed falls below this, mph.
    pub below_mph: f32,
}

impl SpoilerMotion {
    /// The turn as an axis (unit, engine space) and an angle in degrees, `None` for no turn.
    pub fn rotation_axis_angle(&self) -> Option<([f32; 3], f32)> {
        let [x, y, z, w] = self.rotation;
        let sine = (x * x + y * y + z * z).sqrt();
        let angle = 2.0 * sine.atan2(w).to_degrees();
        (angle.abs() >= MIN_TURN_DEG && sine > 1e-6).then(|| ([x / sine, y / sine, z / sine], angle))
    }
}

/// The spoiler motion of the car in `bundle`. `None` for a car without a spoiler machine, with a machine that is more than
/// one speed-driven state, or whose animation does not move the skeleton's `SpoilerHinge`.
pub fn spoiler_motion(bundle: &Bundle, skeleton: &Skeleton) -> Option<SpoilerMotion> {
    let bone = skeleton.find("SpoilerHinge")?;
    let kind = ResourceKind::Animation.id();
    for index in (0..bundle.entries().len()).filter(|&at| bundle.entries()[at].kind == HSM_KIND) {
        let machine = bundle.resource(index).ok()?;
        let block = &machine.blocks[0];
        let strings = ascii_runs(block);
        if !strings.iter().any(|text| text.contains("SpoilerAnimation")) {
            continue;
        }
        let (above_mph, below_mph) = speeds(&conditions(block))?;
        let ids: Vec<u64> = strings
            .iter()
            .filter(|text| text.len() >= 5 && text.bytes().all(|b| b.is_ascii_digit()))
            .filter_map(|text| text.parse().ok())
            .collect();
        let [number] = ids[..] else { return None };
        let at = bundle.entries().iter().position(|e| e.kind == kind && e.id & ID_MASK == number)?;
        let animation = bundle.resource(at).ok()?;
        return movement(&animation.blocks[0], bone).map(|(translation, rotation, seconds)| SpoilerMotion {
            bone,
            translation,
            rotation,
            seconds,
            above_mph,
            below_mph,
        });
    }
    None
}

/// Every run of two or more printable ASCII characters.
fn ascii_runs(block: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut run = Vec::new();
    for &byte in block.iter().chain(std::iter::once(&0)) {
        if (32..127).contains(&byte) {
            run.push(byte);
            continue;
        }
        if run.len() >= 2 {
            out.push(String::from_utf8_lossy(&run).into_owned());
        }
        run.clear();
    }
    out
}

fn word(block: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(block.get(at..at + 4)?.try_into().ok()?))
}

fn half(block: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(block.get(at..at + 2)?.try_into().ok()?))
}

/// The NUL-terminated text at `at`, when it is short printable ASCII.
fn text_at(block: &[u8], at: usize) -> Option<String> {
    let tail = block.get(at..)?;
    let end = tail.iter().position(|&b| b == 0)?;
    let text = &tail[..end];
    (!text.is_empty() && text.len() < 48 && text.iter().all(|b| (32..127).contains(b)))
        .then(|| String::from_utf8_lossy(text).into_owned())
}

/// Whether `rows` rows of a property table start at `at`: 16 bytes each, a 64-bit key whose low byte is 0 or 0x80, the
/// offset of the value cell, a count and a flag word of 0 or 1. Gives each row's key and cell.
fn rows_at(block: &[u8], at: usize, rows: usize) -> Option<Vec<(u64, usize)>> {
    if at % 4 != 0 || at + 16 * rows > block.len() {
        return None;
    }
    let mut out = Vec::with_capacity(rows);
    for n in 0..rows {
        let row = at + 16 * n;
        let key = u64::from(word(block, row)?) | (u64::from(word(block, row + 4)?) << 32);
        let (cell, flags) = (word(block, row + 8)? as usize, half(block, row + 14)?);
        if !matches!(key & 0xff, 0 | 0x80) || key >> 8 == 0 || !(1..block.len()).contains(&cell) || flags > 1 {
            return None;
        }
        out.push((key, cell));
    }
    Some(out)
}

/// A condition of the machine: the input it reads, the operator code and the number, as the object's strings spell them.
#[derive(Debug, Clone, PartialEq)]
struct Condition {
    input: String,
    operator: String,
    value: String,
}

/// Every condition object of the machine: a property table of three to five rows that has an input name, an operator and a
/// number.
fn conditions(block: &[u8]) -> Vec<Condition> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 48 <= block.len() {
        let found = (3..=5).find_map(|rows| rows_at(block, at, rows).map(|table| (rows, table)));
        let Some((rows, table)) = found else {
            at += 4;
            continue;
        };
        let value_of = |key: u64| {
            let (_, cell) = *table.iter().find(|(k, _)| *k == key)?;
            text_at(block, word(block, cell)? as usize)
        };
        match (value_of(KEY_INPUT), value_of(KEY_OPERATOR), value_of(KEY_VALUE)) {
            (Some(input), Some(operator), Some(value)) => {
                out.push(Condition { input, operator, value });
                at += 16 * rows;
            }
            _ => at += 4,
        }
    }
    out
}

/// The speeds of a machine that is one animation opened above a speed and closed below a speed: exactly two conditions, both
/// on the speed, one `>` and one `<`.
fn speeds(conditions: &[Condition]) -> Option<(f32, f32)> {
    let [first, second] = conditions else { return None };
    let number = |condition: &Condition| {
        (condition.input == SPEED_INPUT)
            .then(|| condition.value.parse::<f32>().ok())
            .flatten()
            .filter(|value| value.is_finite())
    };
    let (open, close) = match (first.operator.as_str(), second.operator.as_str()) {
        (GREATER, LESS) => (first, second),
        (LESS, GREATER) => (second, first),
        _ => return None,
    };
    Some((number(open)?, number(close)?))
}

/// The movement of `bone` in an animation, between its reference key and its last: translation, rotation quaternion and the
/// animation's length. The sampler of the executable adds, to a bone's rest pose, the interpolation of two
/// keys minus the reference key; so the pose at the end of the animation is the last key less the reference. Read as the
/// executable reads it: the bone maps give the track of a bone (0xFF for none), translations are 16-byte rows by key and
/// track, rotations are 32-bit packed quaternions (format 1) by key and track.
fn movement(block: &[u8], bone: usize) -> Option<([f32; 3], [f32; 4], f32)> {
    let seconds = f32::from_bits(word(block, DURATION_AT)?);
    let (format, reference) = (half(block, FORMAT_AT)?, usize::from(half(block, REFERENCE_AT)?));
    let (keys, bones) = (word(block, KEYS_AT)? as usize, word(block, BONES_AT)? as usize);
    let (rotation_stride, translation_stride) =
        (word(block, ROTATION_STRIDE_AT)? as usize, word(block, TRANSLATION_STRIDE_AT)? as usize);
    let table = |n: usize| word(block, TABLES_AT + 4 * n).map(|at| at as usize);
    let (translations, translation_map, packed, rotation_map) = (table(1)?, table(4)?, table(5)?, table(7)?);
    if keys < 2 || bone >= bones || !(0.01..=60.0).contains(&seconds) || reference >= keys {
        return None;
    }
    let track = |map: usize| block.get(map + bone).copied().filter(|&t| t != 0xff).map(usize::from);
    let mut translation = [0.0f32; 3];
    if let Some(track) = track(translation_map) {
        let row = |key: usize| -> Option<[f32; 3]> {
            let at = translations + key * translation_stride + 16 * track;
            Some([0, 4, 8].map(|k| f32::from_bits(word(block, at + k).unwrap_or(0))))
        };
        let (last, first) = (row(keys - 1)?, row(reference)?);
        translation = [last[0] - first[0], last[1] - first[1], last[2] - first[2]];
    }
    let mut rotation = [0.0, 0.0, 0.0, 1.0];
    if let Some(track) = track(rotation_map) {
        if format != PACKED_FORMAT {
            return None;
        }
        let key = |key: usize| word(block, packed + key * rotation_stride + 4 * track).map(unpack_rotation);
        rotation = relative(key(keys - 1)?, key(reference)?);
    }
    let moved = translation.iter().any(|c| c.abs() >= MIN_TRAVEL);
    let turn = SpoilerMotion { bone, translation, rotation, seconds, above_mph: 0.0, below_mph: 0.0 };
    (moved || turn.rotation_axis_angle().is_some()).then_some((translation, rotation, seconds))
}

/// A packed rotation: the top two bits say which of the four components is left out; the three 10-bit fields below hold
/// the others, in order, as `(field - 511) * 0.001382`; the left-out one is what makes the quaternion unit. The fields are
/// bits 20 to 29, 10 to 19 and 0 to 9.
pub(crate) fn unpack_rotation(packed: u32) -> [f32; 4] {
    let left_out = (packed >> 30) as usize;
    let field = |shift: u32| (((packed >> shift) & 0x3ff) as f32 - PACK_CENTRE) * PACK_SCALE;
    let stored = [field(20), field(10), field(0)];
    let w = (1.0 - stored.iter().map(|c| c * c).sum::<f32>()).max(0.0).sqrt();
    let mut out = [0.0; 4];
    let mut next = stored.into_iter();
    for (lane, slot) in out.iter_mut().enumerate() {
        *slot = if lane == left_out { w } else { next.next().unwrap_or(0.0) };
    }
    out
}

/// `last * inverse(reference)` for unit quaternions (x, y, z, w).
fn relative(last: [f32; 4], reference: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = last;
    let [bx, by, bz, bw] = [-reference[0], -reference[1], -reference[2], reference[3]];
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn condition(input: &str, operator: &str, value: &str) -> Condition {
        Condition { input: input.into(), operator: operator.into(), value: value.into() }
    }

    #[test]
    fn a_greater_and_a_less_condition_on_the_speed_are_the_two_thresholds() {
        let found = [condition("speedinmph", "3", "40"), condition("speedinmph", "2", "70")];
        assert_eq!(speeds(&found), Some((70.0, 40.0)));
        let equal = [condition("speedinmph", "2", "75"), condition("speedinmph", "3", "75")];
        assert_eq!(speeds(&equal), Some((75.0, 75.0)), "the SL65 opens and closes at the same speed");
    }

    #[test]
    fn any_other_machine_is_not_read() {
        let gas = [condition("gas", "0", "0"), condition("speedinmph", "2", "70")];
        assert_eq!(speeds(&gas), None);
        let two_stage = [
            condition("speedinmph", "3", "75"),
            condition("speedinmph", "2", "75"),
            condition("speedinmph", "2", "100"),
        ];
        assert_eq!(speeds(&two_stage), None);
        let brake = [condition("brake", "2", "0"), condition("speedinmph", "3", "40")];
        assert_eq!(speeds(&brake), None);
        assert_eq!(speeds(&[]), None);
    }

    /// Append a condition's property table (four rows), their value cells and their strings.
    fn put_condition(block: &mut Vec<u8>, input: &str, operator: &str, value: &str) {
        while block.len() % 16 != 0 {
            block.push(0);
        }
        let at = block.len();
        let items = [(KEY_VALUE, value), (KEY_INPUT, input), (KEY_OPERATOR, operator), (0xbfc0_8bcb_9100_0000, "0")];
        block.resize(at + 64 + 16, 0);
        for (n, (key, text)) in items.into_iter().enumerate() {
            let (row, cell) = (at + 16 * n, at + 64 + 4 * n);
            let text_at = block.len();
            block.extend_from_slice(text.as_bytes());
            block.push(0);
            block[row..row + 8].copy_from_slice(&key.to_le_bytes());
            block[row + 8..row + 12].copy_from_slice(&(cell as u32).to_le_bytes());
            block[row + 12..row + 14].copy_from_slice(&1u16.to_le_bytes());
            block[row + 14..row + 16].copy_from_slice(&1u16.to_le_bytes());
            block[cell..cell + 4].copy_from_slice(&(text_at as u32).to_le_bytes());
        }
    }

    #[test]
    fn conditions_are_read_from_their_property_tables() {
        let mut block = vec![0u8; 0x40];
        put_condition(&mut block, "speedinmph", "3", "40");
        put_condition(&mut block, "speedinmph", "2", "70");
        let found = conditions(&block);
        assert_eq!(found, vec![condition("speedinmph", "3", "40"), condition("speedinmph", "2", "70")]);
    }

    #[test]
    fn packed_rotations_read_as_the_911s_do() {
        // The 911's spoiler: key 0 (the reference) is the identity, key 1 a 30.5 degree turn about the lateral axis.
        let identity = unpack_rotation(0xDFF7_FDFF);
        assert!(identity[..3].iter().all(|c| c.abs() < 2e-3) && (identity[3] - 1.0).abs() < 1e-4, "{identity:?}");
        let turned = relative(unpack_rotation(0xEBD7_FDFF), identity);
        let motion = SpoilerMotion {
            bone: 0,
            translation: [0.0; 3],
            rotation: turned,
            seconds: 0.6,
            above_mph: 70.0,
            below_mph: 40.0,
        };
        let (axis, angle) = motion.rotation_axis_angle().unwrap();
        assert!((angle - 30.46).abs() < 0.05, "{angle}");
        assert!(axis[0] > 0.999, "about the lateral axis: {axis:?}");
    }

    #[test]
    fn a_quaternion_with_no_turn_has_no_axis() {
        let motion = SpoilerMotion {
            bone: 0,
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            seconds: 1.0,
            above_mph: 1.0,
            below_mph: 1.0,
        };
        assert_eq!(motion.rotation_axis_angle(), None);
    }

    /// An animation block with two keys, one translation track and one rotation track, both for bone 5 of 8.
    fn animation(translations: [[f32; 3]; 2], rotations: [u32; 2]) -> Vec<u8> {
        let mut block = vec![0u8; 0x200];
        let mut put = |at: usize, value: u32| block[at..at + 4].copy_from_slice(&value.to_le_bytes());
        put(KEYS_AT, 2);
        put(BONES_AT, 8);
        put(ROTATION_STRIDE_AT, 4);
        put(TRANSLATION_STRIDE_AT, 16);
        put(DURATION_AT, 0.5f32.to_bits());
        put(FORMAT_AT, 1);
        put(TABLES_AT + 4, 0x80);
        put(TABLES_AT + 4 * 4, 0x100);
        put(TABLES_AT + 4 * 5, 0xC0);
        put(TABLES_AT + 4 * 7, 0x120);
        for (key, row) in translations.iter().enumerate() {
            for (k, value) in row.iter().enumerate() {
                put(0x80 + 16 * key + 4 * k, value.to_bits());
            }
        }
        for (key, packed) in rotations.iter().enumerate() {
            put(0xC0 + 4 * key, *packed);
        }
        block[0x100..0x108].fill(0xff);
        block[0x120..0x128].fill(0xff);
        block[0x105] = 0;
        block[0x125] = 0;
        block
    }

    #[test]
    fn a_bone_moves_by_its_last_key_less_the_reference_and_other_bones_do_not() {
        let block = animation([[0.0; 3], [0.0, 0.0203, -0.07]], [0xDFF7_FDFF, 0xEBD7_FDFF]);
        let (translation, rotation, seconds) = movement(&block, 5).unwrap();
        assert_eq!((translation, seconds), ([0.0, 0.0203, -0.07], 0.5));
        assert!(rotation[0] > 0.2 && rotation[3] > 0.9);
        assert_eq!(movement(&block, 4), None, "a bone with no track does not move");
        assert_eq!(movement(&block, 9), None, "a bone the skeleton has not");
        let still = animation([[0.0; 3]; 2], [0xDFF7_FDFF; 2]);
        assert_eq!(movement(&still, 5), None);
    }

    #[test]
    fn ascii_runs_find_the_machines_strings() {
        let block = b"\x01\x02Speed above 70mph\x00\x00A\x00hi\x00";
        assert_eq!(ascii_runs(block), vec!["Speed above 70mph".to_owned(), "hi".to_owned()]);
    }
}
