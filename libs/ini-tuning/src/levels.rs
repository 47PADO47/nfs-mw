//! Per-level tables: one key with a value for each of `N` levels, each in its own section named by a prefix and the
//! level number. With the prefix `Heat` and `N` 10, the key `CopCap` is read from `[Heat1]` to `[Heat10]`. The names
//! of the sections and keys belong to the caller; this file only walks the levels.

use crate::{Entry, Range, Read, read_number};

/// The section name of level `level` (1-based): the prefix and the number, as in `Heat3`.
pub fn section_name(prefix: &str, level: usize) -> String {
    format!("{prefix}{level}")
}

/// Reads `key` from the section of each level 1 to `N`, clamped to `range`. Each slot is what its own line gave: a
/// level without the section or the key is [`Read::Absent`], so the caller picks its default per level.
pub fn read_levels<const N: usize>(entries: &[Entry], prefix: &str, key: &str, range: Range) -> [Read; N] {
    std::array::from_fn(|i| read_number(entries, &section_name(prefix, i + 1), key, range))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entries;

    const UNIT: Range = Range { min: 0.0, max: 10.0 };

    #[test]
    fn each_level_reads_its_own_section() {
        let text = "[Heat1]\nCap = 2\n[Heat3]\nCap = 7\n[Heat2]\nOther = 1\n";
        let got: [Read; 3] = read_levels(&entries(text), "Heat", "Cap", UNIT);
        assert_eq!(got, [Read::Value(2.0), Read::Absent, Read::Value(7.0)]);
    }

    #[test]
    fn levels_are_clamped_and_malformed_lines_are_kept_apart() {
        let text = "[Heat1]\nCap = 99\n[Heat2]\nCap = lots\n";
        let got: [Read; 2] = read_levels(&entries(text), "Heat", "Cap", UNIT);
        assert_eq!(got, [Read::Clamped(10.0), Read::Malformed]);
    }

    #[test]
    fn the_section_name_joins_prefix_and_level() {
        assert_eq!(section_name("Heat", 10), "Heat10");
    }

    #[test]
    fn a_later_section_line_overrides_an_earlier_one() {
        let text = "[Heat1]\nCap = 1\n[Heat1]\nCap = 4\n";
        let got: [Read; 1] = read_levels(&entries(text), "Heat", "Cap", UNIT);
        assert_eq!(got, [Read::Value(4.0)]);
    }
}
