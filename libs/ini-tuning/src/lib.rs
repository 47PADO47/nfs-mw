//! Reader for INI-style tuning files: `[Section]` headers, `Key = value` lines and `//` comments.
//! Numeric values are checked against an inclusive range. Pure text in, values out: no file access
//! and no knowledge of any game.

pub mod levels;

pub use levels::{read_levels, section_name};

/// One `Key = value` line under its section, with the names and the value as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub section: String,
    pub key: String,
    pub value: String,
}

/// Every key/value line of `text`, in order. Lines outside a section, lines without `=` and empty keys
/// are skipped.
pub fn entries(text: &str) -> Vec<Entry> {
    let mut section = String::new();
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.split("//").next().unwrap_or_default().trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.trim().to_owned();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        if section.is_empty() || key.is_empty() {
            continue;
        }
        out.push(Entry { section: section.clone(), key: key.to_owned(), value: value.trim().to_owned() });
    }
    out
}

/// The last line that sets `section.key`: a later line overrides an earlier one.
pub fn last<'a>(entries: &'a [Entry], section: &str, key: &str) -> Option<&'a Entry> {
    entries.iter().rev().find(|e| e.section == section && e.key == key)
}

/// A switch as the mod writes it: `0`/`1`, or `true`/`false` in any case.
pub fn switch(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

/// A finite decimal number, such as `0.45`.
pub fn number(value: &str) -> Option<f32> {
    value.parse::<f32>().ok().filter(|v| v.is_finite())
}

/// Inclusive bounds for a number. Use `f32::NEG_INFINITY` or `f32::INFINITY` for an open side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub min: f32,
    pub max: f32,
}

/// What a numeric line gave.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Read {
    /// No line sets the key.
    Absent,
    /// The line's value is not a finite number.
    Malformed,
    /// A number outside the range, moved to the nearest bound.
    Clamped(f32),
    /// A number inside the range.
    Value(f32),
}

impl Read {
    /// The usable value, or `None` when the key is absent or malformed. The caller then uses its default.
    pub fn value(self) -> Option<f32> {
        match self {
            Read::Clamped(v) | Read::Value(v) => Some(v),
            Read::Absent | Read::Malformed => None,
        }
    }
}

/// Reads `section.key` as a number clamped to `range`.
pub fn read_number(entries: &[Entry], section: &str, key: &str, range: Range) -> Read {
    let Some(entry) = last(entries, section, key) else { return Read::Absent };
    let Some(value) = number(&entry.value) else { return Read::Malformed };
    let clamped = value.clamp(range.min, range.max);
    match clamped == value {
        true => Read::Value(value),
        false => Read::Clamped(clamped),
    }
}

/// Reads `section.key` as a comma-separated list of up to `N` numbers, part `i` clamped to `ranges[i]`. A key that is
/// absent gives [`Read::Absent`] in every slot. A list shorter than `N` leaves its last slots [`Read::Absent`], so the
/// caller keeps its default for them; parts past the `N`th are ignored. An empty or non-numeric part is
/// [`Read::Malformed`] for its slot only.
pub fn read_numbers<const N: usize>(entries: &[Entry], section: &str, key: &str, ranges: [Range; N]) -> [Read; N] {
    let Some(entry) = last(entries, section, key) else { return [Read::Absent; N] };
    let mut parts = entry.value.split(',').map(str::trim);
    std::array::from_fn(|i| {
        let Some(part) = parts.next() else { return Read::Absent };
        let Some(value) = number(part) else { return Read::Malformed };
        let clamped = value.clamp(ranges[i].min, ranges[i].max);
        match clamped == value {
            true => Read::Value(value),
            false => Read::Clamped(clamped),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNIT: Range = Range { min: 0.0, max: 10.0 };

    #[test]
    fn reads_sections_keys_and_strips_comments() {
        let text = "[Gameplay]\nNoRevLimiter = 1   // removes the limiter\n\n[Misc]\nSkipMovies=0\n";
        let got = entries(text);
        assert_eq!(got.len(), 2);
        assert_eq!(
            (got[0].section.as_str(), got[0].key.as_str(), got[0].value.as_str()),
            ("Gameplay", "NoRevLimiter", "1")
        );
        assert_eq!((got[1].section.as_str(), got[1].value.as_str()), ("Misc", "0"));
    }

    #[test]
    fn skips_keys_outside_a_section_and_malformed_lines() {
        let text = "Orphan = 1\n[A]\nno equals here\n = 3\nKey = 2\n";
        assert_eq!(entries(text), vec![Entry { section: "A".into(), key: "Key".into(), value: "2".into() }]);
    }

    #[test]
    fn a_later_line_overrides_an_earlier_one() {
        let got = entries("[A]\nK = 1\nK = 2\n");
        assert_eq!(last(&got, "A", "K").map(|e| e.value.as_str()), Some("2"));
        assert!(last(&got, "A", "Other").is_none());
    }

    #[test]
    fn switches_accept_numbers_and_words() {
        assert_eq!(switch("1"), Some(true));
        assert_eq!(switch("False"), Some(false));
        assert_eq!(switch("2"), None);
    }

    #[test]
    fn numbers_must_be_finite() {
        assert_eq!(number("0.45"), Some(0.45));
        assert_eq!(number("inf"), None);
        assert_eq!(number("nan"), None);
    }

    #[test]
    fn read_number_distinguishes_absent_malformed_clamped_and_valid() {
        let got = entries("[S]\nValid = 4\nHigh = 99\nLow = -3\nBad = lots\n");
        assert_eq!(read_number(&got, "S", "Valid", UNIT), Read::Value(4.0));
        assert_eq!(read_number(&got, "S", "High", UNIT), Read::Clamped(10.0));
        assert_eq!(read_number(&got, "S", "Low", UNIT), Read::Clamped(0.0));
        assert_eq!(read_number(&got, "S", "Bad", UNIT), Read::Malformed);
        assert_eq!(read_number(&got, "S", "Missing", UNIT), Read::Absent);
    }

    #[test]
    fn a_list_reads_each_part_with_its_own_range() {
        let ranges = [UNIT, Range { min: -5.0, max: 5.0 }, UNIT];
        let got = entries(
            "[S]
L = 4, -9 ,x
Short = 1
One = 1,2,3,4,5
Empty =
",
        );
        assert_eq!(read_numbers(&got, "S", "L", ranges), [Read::Value(4.0), Read::Clamped(-5.0), Read::Malformed]);
        assert_eq!(read_numbers(&got, "S", "Short", ranges), [Read::Value(1.0), Read::Absent, Read::Absent]);
        assert_eq!(
            read_numbers(&got, "S", "One", [UNIT, UNIT]),
            [Read::Value(1.0), Read::Value(2.0)],
            "parts past the last slot are ignored"
        );
        assert_eq!(read_numbers(&got, "S", "Empty", [UNIT]), [Read::Malformed]);
        assert_eq!(read_numbers(&got, "S", "Missing", ranges), [Read::Absent; 3]);
    }

    #[test]
    fn only_clamped_and_valid_reads_have_a_value() {
        assert_eq!(Read::Value(1.0).value(), Some(1.0));
        assert_eq!(Read::Clamped(2.0).value(), Some(2.0));
        assert_eq!(Read::Absent.value(), None);
        assert_eq!(Read::Malformed.value(), None);
    }
}
