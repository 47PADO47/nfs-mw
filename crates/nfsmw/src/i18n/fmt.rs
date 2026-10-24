//! Numbers as the current language writes them (`docs/specs/localization.md` section 4.3). Times keep the original's own
//! fixed formats (`m:ss.cc`), which do not change with the language.

use super::t;

/// A whole number with the language's thousands separator (`20,000` in English): the game's `%$d`.
pub fn grouped(value: i64) -> String {
    group(value, &t!("group-separator"))
}

/// `1st`, `2nd`, `23rd` in English.
pub fn ordinal(n: u32) -> String {
    t!("format-ordinal", n = n)
}

/// `50%` in English.
pub fn percent(n: i64) -> String {
    t!("format-percent", n = n)
}

fn group(value: i64, separator: &str) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 * separator.len() + 1);
    if value < 0 {
        out.push('-');
    }
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push_str(separator);
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_in_threes() {
        assert_eq!(group(0, ","), "0");
        assert_eq!(group(999, ","), "999");
        assert_eq!(group(20_000, ","), "20,000");
        assert_eq!(group(-1_234_567, "."), "-1.234.567");
    }

    #[test]
    fn english_formats() {
        assert_eq!(grouped(1500), "1,500");
        assert_eq!(ordinal(1), "1st");
        assert_eq!(ordinal(12), "12th");
        assert_eq!(ordinal(22), "22nd");
        assert_eq!(ordinal(103), "103rd");
        assert_eq!(percent(80), "80%");
    }
}
