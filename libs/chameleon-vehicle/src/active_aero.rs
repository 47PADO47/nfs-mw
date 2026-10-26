//! The active spoiler of a 2012 car. Spec: `docs/formats/chameleon-active-aero.md`.
//!
//! Some cars raise a spoiler when they are fast. The bundle holds a state machine (`HSMData`, resource type 0x501) named
//! `<car>_SpoilerAnimation`: a condition on the car's speed (`speedinmph`, the engine's own input name) that plays an
//! animation of the skeleton, `Spoiler Anim`. The state names, the speeds and the ids of the animations are strings of the
//! resource; the animation (type 0xB3) holds, per animated bone, a translation at each key. A car whose state machine
//! also reads the brake (the Bugatti Veyron, the McLaren MP4-12C, the Hennessey Venom) is not read here.

use chameleon_bundle::{Bundle, ResourceKind};

/// The resource type of a state machine.
const HSM_KIND: u32 = 0x501;
/// Where an animation keeps its length in seconds, the number of keys, and the offsets of the key tables.
const DURATION_AT: usize = 0x30;
const KEYS_AT: usize = 4;
const TRANSLATIONS_AT: usize = 15 * 4;
const TRANSLATIONS_END_AT: usize = 16 * 4;
/// The id of a resource without its namespace byte: what the state machine's strings call an animation.
const ID_MASK: u64 = 0x00FF_FFFF_FFFF;
/// Smallest movement taken for one, in metres.
const MIN_TRAVEL: f32 = 1e-3;
/// How far below its deploy speed a spoiler comes down when the data names no lower speed, mph.
const DEFAULT_HYSTERESIS_MPH: f32 = 10.0;

/// What a spoiler does: how far it moves, how long it takes and at which speeds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpoilerMotion {
    /// The movement of the spoiler's bone between the first and the last key, metres, in the engine's space (x left, y up,
    /// z forward).
    pub offset: [f32; 3],
    /// Length of the animation, seconds.
    pub seconds: f32,
    /// The spoiler goes up at this speed and above, mph.
    pub above_mph: f32,
    /// It comes down below this speed, mph.
    pub below_mph: f32,
}

/// The spoiler motion of the car in `bundle`. `None` for a car without a spoiler animation, with a state machine that also
/// reads the brake, or whose animation moves nothing.
pub fn spoiler_motion(bundle: &Bundle) -> Option<SpoilerMotion> {
    let kind = ResourceKind::Animation.id();
    for index in (0..bundle.entries().len()).filter(|&at| bundle.entries()[at].kind == HSM_KIND) {
        let machine = bundle.resource(index).ok()?;
        let strings = ascii_runs(&machine.blocks[0]);
        if !strings.iter().any(|text| text.contains("SpoilerAnimation")) {
            continue;
        }
        if strings.iter().any(|text| text.to_ascii_lowercase().contains("brake")) {
            return None;
        }
        let (above_mph, below_mph) = speeds(&strings)?;
        let ids = strings.iter().filter(|text| text.len() >= 5 && text.bytes().all(|b| b.is_ascii_digit()));
        for number in ids.filter_map(|text| text.parse::<u64>().ok()) {
            let Some(at) = bundle.entries().iter().position(|e| e.kind == kind && e.id & ID_MASK == number) else {
                continue;
            };
            let animation = bundle.resource(at).ok()?;
            let Some((offset, seconds)) = translation(&animation.blocks[0]) else { continue };
            return Some(SpoilerMotion { offset, seconds, above_mph, below_mph });
        }
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

fn first_number(text: &str) -> Option<f32> {
    let digits: String = text.chars().skip_while(|c| !c.is_ascii_digit()).take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// The speeds of the state machine: the lowest speed a condition name says the car is `above` (or the only one it names)
/// raises the spoiler; the highest plain number below it, one of the machine's constants, is where it comes down, else
/// ten mph under.
fn speeds(strings: &[String]) -> Option<(f32, f32)> {
    let named: Vec<&str> =
        strings.iter().map(String::as_str).filter(|text| text.starts_with("Speed") && !text.contains("OR")).collect();
    let lowest = |only_above: bool| {
        named
            .iter()
            .filter(|text| !only_above || text.contains("above"))
            .filter_map(|text| first_number(text))
            .reduce(f32::min)
    };
    let up = lowest(true).or_else(|| lowest(false)).filter(|up| *up > 0.0)?;
    let below = strings
        .iter()
        .filter(|text| text.len() <= 3 && text.bytes().all(|b| b.is_ascii_digit()))
        .filter_map(|text| text.parse::<f32>().ok())
        .filter(|value| *value > 0.0 && *value < up)
        .fold(0.0f32, f32::max);
    Some((up, if below > 0.0 { below } else { (up - DEFAULT_HYSTERESIS_MPH).max(0.0) }))
}

/// The movement of the first animated track of an animation, and the animation's length. The key tables hold, key by key,
/// one 16-byte translation per animated track.
fn translation(block: &[u8]) -> Option<([f32; 3], f32)> {
    let word = |at: usize| block.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let seconds = f32::from_bits(word(DURATION_AT)?);
    let keys = word(KEYS_AT)? as usize;
    let (start, end) = (word(TRANSLATIONS_AT)? as usize, word(TRANSLATIONS_END_AT)? as usize);
    let count = end.checked_sub(start)? / 16;
    if keys < 2 || count == 0 || count % keys != 0 || !(0.01..=60.0).contains(&seconds) {
        return None;
    }
    let tracks = count / keys;
    let entry = |n: usize| -> Option<[f32; 3]> {
        let at = start + 16 * n;
        Some([0, 4, 8].map(|k| f32::from_bits(word(at + k).unwrap_or(0))))
    };
    for track in 0..tracks {
        let (first, last) = (entry(track)?, entry((keys - 1) * tracks + track)?);
        let delta = [last[0] - first[0], last[1] - first[1], last[2] - first[2]];
        if delta.iter().all(|c| c.is_finite()) && delta.iter().any(|c| c.abs() >= MIN_TRAVEL) {
            return Some((delta, seconds));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn the_911_goes_up_at_70_and_down_at_40() {
        let found = strings(&["Porsche_911CarreraS_2012_SpoilerAnimation", "Speed above 70mph", "Spoiler Anim", "904078", "40", "speedinmph", "False", "0.25", "70"]);
        assert_eq!(speeds(&found), Some((70.0, 40.0)));
    }

    #[test]
    fn two_stage_machines_start_at_the_lower_speed_and_a_lone_number_has_a_default_drop() {
        let lp700 = strings(&["Speed above 80mph", "Speed above 100mph", "Speed under 100mph", "100", "50", "80"]);
        assert_eq!(speeds(&lp700), Some((80.0, 50.0)));
        let sl65 = strings(&["Speed 75mph", "75", "speedinmph"]);
        assert_eq!(speeds(&sl65), Some((75.0, 65.0)));
        let nothing = strings(&["Spoiler Anim"]);
        assert_eq!(speeds(&nothing), None);
    }

    /// An animation block with `keys` keys of `tracks` tracks, translations given key by key.
    fn animation(keys: u32, tracks: usize, seconds: f32, entries: &[[f32; 3]]) -> Vec<u8> {
        let start = 0x70usize;
        let end = start + 16 * entries.len();
        let mut block = vec![0u8; end + 0x40];
        let mut put = |at: usize, value: u32| block[at..at + 4].copy_from_slice(&value.to_le_bytes());
        put(KEYS_AT, keys);
        put(DURATION_AT, seconds.to_bits());
        put(TRANSLATIONS_AT, start as u32);
        put(TRANSLATIONS_END_AT, end as u32);
        assert_eq!(entries.len(), keys as usize * tracks);
        for (n, entry) in entries.iter().enumerate() {
            for (k, value) in entry.iter().enumerate() {
                put(start + 16 * n + 4 * k, value.to_bits());
            }
        }
        block
    }

    #[test]
    fn a_one_track_animation_moves_by_its_last_key() {
        let block = animation(2, 1, 0.612, &[[0.0; 3], [0.0, 0.0203, -0.07]]);
        assert_eq!(translation(&block), Some(([0.0, 0.0203, -0.07], 0.612)));
    }

    #[test]
    fn the_first_track_that_moves_is_the_spoiler() {
        // Two tracks, two keys: the first never moves, the second goes up and back.
        let block = animation(2, 2, 0.5, &[[0.0, 0.0133, 0.0061], [0.0; 3], [0.0, 0.0133, 0.0061], [0.0, 0.09, -0.06]]);
        assert_eq!(translation(&block), Some(([0.0, 0.09, -0.06], 0.5)));
    }

    #[test]
    fn an_animation_that_moves_nothing_or_is_malformed_is_not_a_motion() {
        assert_eq!(translation(&animation(2, 1, 0.5, &[[0.0; 3], [0.0; 3]])), None);
        assert_eq!(translation(&animation(2, 1, 0.0, &[[0.0; 3], [0.0, 1.0, 0.0]])), None, "no length");
        assert_eq!(translation(&[0u8; 0x20]), None);
    }

    #[test]
    fn ascii_runs_find_the_machines_strings() {
        let block = b"\x01\x02Speed above 70mph\x00\x00A\x00hi\x00";
        assert_eq!(ascii_runs(block), vec!["Speed above 70mph".to_owned(), "hi".to_owned()]);
    }
}
