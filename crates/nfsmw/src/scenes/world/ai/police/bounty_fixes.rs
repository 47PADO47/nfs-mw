//! The bounty bug fixes of a cop takedown, on by default (`docs/specs/ai-pursuit-heat.md` section 5.5). Each rule has
//! two answers: the original's, bugs and all, and the fixed one. The fixes follow what the common Extra Options mod
//! changes in the original game; this file holds only the resulting rules.

/// The last entry of the 10-entry `RepPointsForDestroying` array.
const LAST_ENTRY: u32 = 9;
/// Destruction points at or below this are the data's placeholders (the helicopter's array counts 1 to 10).
const PLACEHOLDER_MAX: u32 = 10;
/// What a placeholder scores with the fixes on.
const PLACEHOLDER_POINTS: u32 = 100_000;
/// A destruction window this long or longer (heats 8 and 10, and the race table's heats 6 to 10) ...
const LONG_WINDOW_SECS: f32 = 120.0;
/// ... is this long with the fixes on.
const FIXED_WINDOW_SECS: f32 = 10.0;

/// The `RepPointsForDestroying` entry a wreck at heat `heat` reads. The original reads entry `heat`, so heat 10 runs
/// off the array and scores nothing; the fix reads the last entry from heat 9 up.
pub(super) fn points_entry(heat: u32, fixed: bool) -> u32 {
    match fixed {
        true => heat.min(LAST_ENTRY),
        false => heat,
    }
}

/// The destruction points a wreck banks before the multiplier. The fix replaces a placeholder (1 to 10 points, the
/// helicopter's) with 100 000; 0 (a car the table lacks) stays 0.
pub(super) fn points(points: u32, fixed: bool) -> u32 {
    match fixed && (1..=PLACEHOLDER_MAX).contains(&points) {
        true => PLACEHOLDER_POINTS,
        false => points,
    }
}

/// The destruction window a wreck starts, seconds. The fix shortens a window of two minutes or more to 10 s.
pub(super) fn window_secs(secs: f32, fixed: bool) -> f32 {
    match fixed && secs >= LONG_WINDOW_SECS {
        true => FIXED_WINDOW_SECS,
        false => secs,
    }
}

/// The vehicle whose takedown line the HUD shows. The original has no line for the helicopter; the fix gives it the
/// Federal Pursuit Vehicle line.
pub(super) fn line_car(car: &str, fixed: bool) -> &str {
    match fixed && car.eq_ignore_ascii_case(nfsmw_data::pursuit::HELICOPTER) {
        true => "copsporthench",
        false => car,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_original_reads_entry_heat_and_the_fix_stops_at_the_last_entry() {
        assert_eq!(points_entry(5, false), 5);
        assert_eq!(points_entry(10, false), 10, "runs off the 10-entry array");
        assert_eq!(points_entry(5, true), 5);
        assert_eq!(points_entry(10, true), 9);
    }

    #[test]
    fn only_placeholder_points_are_raised() {
        assert_eq!(points(6, false), 6);
        assert_eq!(points(6, true), 100_000);
        assert_eq!(points(10, true), 100_000);
        assert_eq!(points(250, true), 250);
        assert_eq!(points(0, true), 0, "a car without points stays at none");
    }

    #[test]
    fn only_a_window_of_two_minutes_or_more_is_shortened() {
        assert_eq!(window_secs(120.0, false), 120.0);
        assert_eq!(window_secs(120.0, true), 10.0);
        assert_eq!(window_secs(10.0, true), 10.0);
    }

    #[test]
    fn the_helicopter_takes_the_federal_line_only_with_the_fix() {
        assert_eq!(line_car("copheli", false), "copheli");
        assert_eq!(line_car("copheli", true), "copsporthench");
        assert_eq!(line_car("copgto", true), "copgto");
    }
}
