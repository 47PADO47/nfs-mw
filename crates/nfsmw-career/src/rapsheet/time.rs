//! The engine time formatter used for pursuit lengths (`docs/specs/career-rapsheet-2.md` section 1.3).
//!
//! Ticks are 1/4000 s. The default separators (`:` and `.`) are used; some languages use `'`, `"`, `.` or `,`, which
//! a caller can substitute in the result (the spec does not give the per-language table).

/// Ticks per second.
pub const TICKS_PER_SECOND: u32 = 4000;
/// The value the formatter prints as zero besides 0.
pub const ZERO_TICKS_ALT: u32 = 0x7FFF_FFFF;
/// Flag bit 0: prefix `-`.
pub const FLAG_MINUS: u32 = 0x1;
/// Flag bit 1: prefix `+`.
pub const FLAG_PLUS: u32 = 0x2;
/// Flag bit `0x10`: drop the last three characters (`.cc`) in the last three cases.
pub const FLAG_NO_FRACTION: u32 = 0x10;
/// The flags of the top 5 and detail pages.
pub const FLAGS_PAGE: u32 = 0;
/// The flags of the ranking detail page.
pub const FLAGS_RANKING: u32 = FLAG_NO_FRACTION;

/// Formats a tick count:
///
/// | case | text |
/// |---|---|
/// | hours not 0 | `H:MM:SS`, the hour field two wide with a leading blank |
/// | minutes not 0 | `MM:SS.cc`, minutes two wide with a leading blank |
/// | seconds not 0 | `S.cc` |
/// | otherwise | `0.cc` |
///
/// `cc` is `((ticks % 4000) * 1000 / 4000) / 10`. The hour case has no fraction. Ticks 0 and `0x7FFFFFFF` print as
/// zero. The spec writes the seconds case as `SS.cc` without saying whether a single digit is padded; it is not
/// padded here (unconfirmed), nor is the minutes field of the hour case.
pub fn format_ticks(ticks: u32, flags: u32) -> String {
    let ticks = match ticks {
        ZERO_TICKS_ALT => 0,
        t => t,
    };
    let total_s = ticks / TICKS_PER_SECOND;
    let (h, m, s) = (total_s / 3600, (total_s % 3600) / 60, total_s % 60);
    let cc = ((ticks % TICKS_PER_SECOND) * 1000 / TICKS_PER_SECOND) / 10;
    let fraction = match flags & FLAG_NO_FRACTION {
        0 => format!(".{cc:02}"),
        _ => String::new(),
    };
    let body = match (h, m, s) {
        (0, 0, 0) => format!("0{fraction}"),
        (0, 0, s) => format!("{s}{fraction}"),
        (0, m, s) => format!("{m:>2}:{s:02}{fraction}"),
        (h, m, s) => format!("{h:>2}:{m:02}:{s:02}"),
    };
    let sign = match (flags & FLAG_MINUS != 0, flags & FLAG_PLUS != 0) {
        (true, _) => "-",
        (false, true) => "+",
        (false, false) => "",
    };
    format!("{sign}{body}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: u32 = 4000;

    #[test]
    fn the_four_cases() {
        assert_eq!(format_ticks(0, 0), "0.00");
        assert_eq!(format_ticks(0x7FFF_FFFF, 0), "0.00");
        assert_eq!(format_ticks(2 * S + 2000, 0), "2.50");
        assert_eq!(format_ticks(59 * S, 0), "59.00");
        assert_eq!(format_ticks(60 * S, 0), " 1:00.00");
        assert_eq!(format_ticks(12 * 60 * S + 34 * S + 1000, 0), "12:34.25");
        assert_eq!(format_ticks(3600 * S, 0), " 1:00:00");
        assert_eq!(format_ticks(3600 * S + 61 * S + 3999, 0), " 1:01:01");
    }

    #[test]
    fn the_hundredths_truncate() {
        assert_eq!(format_ticks(S + 39, 0), "1.00", "39 ticks = 9.75 ms");
        assert_eq!(format_ticks(S + 40, 0), "1.01");
        assert_eq!(format_ticks(S + 3999, 0), "1.99");
    }

    #[test]
    fn the_no_fraction_flag_drops_the_hundredths_but_not_the_hour_case() {
        assert_eq!(format_ticks(0, FLAGS_RANKING), "0");
        assert_eq!(format_ticks(59 * S + 2000, FLAGS_RANKING), "59");
        assert_eq!(format_ticks(61 * S, FLAGS_RANKING), " 1:01");
        assert_eq!(format_ticks(3601 * S, FLAGS_RANKING), " 1:00:01");
    }

    #[test]
    fn the_sign_flags_prefix() {
        assert_eq!(format_ticks(2 * S, FLAG_MINUS), "-2.00");
        assert_eq!(format_ticks(2 * S, FLAG_PLUS), "+2.00");
        assert_eq!(format_ticks(2 * S, FLAG_MINUS | FLAG_PLUS), "-2.00");
    }

    #[test]
    fn the_thresholds_of_the_rankings_print_as_times() {
        // 14,400 s is exactly four hours.
        assert_eq!(format_ticks(14_400 * S, FLAGS_RANKING), " 4:00:00");
        assert_eq!(format_ticks(1800 * S, FLAGS_RANKING), "30:00");
    }
}
