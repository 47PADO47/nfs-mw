//! The words of the screen: the goal of an event as the descriptions print it, and the line that says what opens a
//! locked event (`UIQRChallengeSeries::RefreshHeader`).

use blackbox_text::label_hash;

use super::board::Entry;

/// The label of the header of an event (`cFrontendDatabase::GetChallengeHeaderHash`).
pub fn header_label(tag: i32) -> u32 {
    label_hash(&format!("CHALLENGE_SERIES_DESC_{tag:02}_HEADER"))
}

/// The label of the description of an event (`GetChallengeDescHash`).
pub fn description_label(tag: i32) -> u32 {
    label_hash(&format!("CHALLENGE_SERIES_DESC_{tag:02}"))
}

/// The two lines that say what opens a locked event: both name challenges by their place in the list.
const UNLOCK_EITHER: &str = "CHALLENGE_SERIES_UNLOCK_MSG_01";
const UNLOCK_ONE: &str = "CHALLENGE_SERIES_UNLOCK_MSG_02";

/// What a locked event says: a label of the language table and the numbers its `%d` stand for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockedLine {
    pub label: u32,
    pub numbers: Vec<usize>,
}

/// The line of the locked event at `index` (counted from 0). The original derives it from the position alone, as the
/// ladder was laid out: from the third entry on, every entry is opened by one of the two before the block of five it
/// sits in, and the last ones by their predecessor.
pub fn locked_line(index: usize) -> LockedLine {
    let either = |a: usize, b: usize| LockedLine { label: label_hash(UNLOCK_EITHER), numbers: vec![a, b] };
    let one = |a: usize| LockedLine { label: label_hash(UNLOCK_ONE), numbers: vec![a] };
    if index >= 61 {
        return one(index);
    }
    let place = (index + 1) % 5;
    let page = (index + 1) / 5;
    match place {
        1 | 2 => one(page * 5),
        3 | 4 => either(page * 5 + 1, page * 5 + 2),
        _ => either((page - 1) * 5 + 3, (page - 1) * 5 + 4),
    }
}

/// The goal as the descriptions print it (`%$0.0f`): a time in minutes for the time statistics, else the number with
/// its thousands separated by commas **[unconfirmed]** (the `$` flag of the original's printf was not read).
pub fn goal_text(entry: &Entry) -> String {
    let goal = match entry.kind.time_goal() {
        true => entry.goal / 60.0,
        false => entry.goal,
    };
    group_digits(goal.round() as i64)
}

/// A whole number with commas between the thousands.
pub fn group_digits(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    match value < 0 {
        true => format!("-{out}"),
        false => out,
    }
}

/// A language string with its `%s` and `%d` places filled in order from `args`. A place with no argument is left empty.
pub fn fill(template: &str, args: &[String]) -> String {
    let mut out = String::new();
    let mut rest = template;
    let mut next = args.iter();
    while let Some(at) = rest.find('%') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let Some(kind) = after.chars().next().filter(|c| matches!(c, 's' | 'd')) else {
            out.push('%');
            rest = after;
            continue;
        };
        out.push_str(next.next().map_or("", String::as_str));
        rest = &after[kind.len_utf8()..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::challenge_series::kinds::{Kind, MILESTONES};
    use crate::frontend::challenge_series::view::Crop;

    fn entry(kind: Kind, goal: f32) -> Entry {
        Entry {
            key: 0,
            name: String::new(),
            event: String::new(),
            kind,
            tag: 1,
            goal,
            open_at_start: false,
            unlocks: Vec::new(),
            crop: Crop::WHOLE,
            map_texture: 0,
        }
    }

    #[test]
    fn labels_are_named_by_the_two_digit_tag() {
        assert_eq!(header_label(31), 0x68031d75, "BURGER KING Challenge");
        assert_eq!(header_label(70), 0x4db30c98, "Black Edition Challenge");
        assert_eq!(description_label(32), 0x5614134e);
        assert_eq!(description_label(1), 0x561412ea);
    }

    #[test]
    fn the_lock_lines_follow_the_laid_out_ladder() {
        assert_eq!(
            locked_line(1).numbers,
            [0],
            "the first block's second entry names a place 0 (never shown: it starts open)"
        );
        let third = locked_line(2);
        assert_eq!((third.label, third.numbers), (label_hash(UNLOCK_EITHER), vec![1, 2]));
        assert_eq!(locked_line(4).numbers, [3, 4], "the fifth entry is opened by the third or fourth");
        assert_eq!(locked_line(5).numbers, [5]);
        assert_eq!(locked_line(69).numbers, [69]);
        assert_eq!(locked_line(70).numbers, [70], "the line of the screenshot: beating challenge 70");
        assert_eq!(locked_line(70).label, 0xced8931e);
        assert_eq!(locked_line(2).label, 0xced8931d);
    }

    #[test]
    fn goals_print_in_minutes_or_with_separated_thousands() {
        let m = |name: &str| *MILESTONES.iter().find(|m| m.name == name).unwrap();
        assert_eq!(goal_text(&entry(Kind::Pursuit(m("pursuit_length")), 600.0)), "10");
        assert_eq!(goal_text(&entry(Kind::Pursuit(m("cost_to_state_in_pursuit")), 200_000.0)), "200,000");
        assert_eq!(goal_text(&entry(Kind::Pursuit(m("bounty_in_pursuit")), 1_000_000.0)), "1,000,000");
        assert_eq!(goal_text(&entry(Kind::Pursuit(m("cops_damaged")), 5.0)), "5");
        assert_eq!(group_digits(999), "999");
        assert_eq!(group_digits(1000), "1,000");
    }

    #[test]
    fn places_are_filled_in_order_and_a_lone_percent_stays() {
        assert_eq!(fill("beating challenge %d or %d.", &["1".into(), "2".into()]), "beating challenge 1 or 2.");
        assert_eq!(fill("at least %s minute", &["10".into()]), "at least 10 minute");
        assert_eq!(fill("100% sure %s", &["x".into()]), "100% sure x");
        assert_eq!(fill("%s and %s", &["a".into()]), "a and ");
    }
}
