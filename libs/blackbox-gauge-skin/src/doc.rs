//! A parsed ini text with typed lookups. A missing or unreadable key reads as the default the caller gives, as the plugin's
//! reader gives 0, 0.0 or empty text for a key that is not there.

use ini_tuning::Entry;

use crate::value::{Colour, parse_float, parse_int};

/// The key/value lines of one ini text.
#[derive(Debug, Clone, Default)]
pub struct Doc {
    entries: Vec<Entry>,
}

impl Doc {
    /// Reads `[Section]` headers and `Key = value` lines. `//` starts a comment; a line without `=` (such as a `#` comment)
    /// is skipped.
    pub fn parse(text: &str) -> Self {
        Self { entries: ini_tuning::entries(text) }
    }

    fn raw(&self, section: &str, key: &str) -> Option<&str> {
        ini_tuning::last(&self.entries, section, key).map(|entry| entry.value.as_str())
    }

    pub fn int(&self, section: &str, key: &str, default: i64) -> i64 {
        self.raw(section, key).and_then(parse_int).unwrap_or(default)
    }

    pub fn float(&self, section: &str, key: &str, default: f32) -> f32 {
        self.raw(section, key).and_then(parse_float).unwrap_or(default)
    }

    /// A switch: any non-zero integer is on.
    pub fn flag(&self, section: &str, key: &str, default: bool) -> bool {
        self.int(section, key, i64::from(default)) != 0
    }

    pub fn colour(&self, section: &str, key: &str) -> Colour {
        Colour(self.int(section, key, 0) as u32)
    }

    /// The value as written, trimmed; empty when the key is not there.
    pub fn text(&self, section: &str, key: &str) -> String {
        self.raw(section, key).unwrap_or_default().trim().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "#a comment\n[TACHOMETER_GAUGE]\nEnabled = 1\nSize = 330\nColor = 0xFF8ec8ff\nArrowTexture = tach-arrow.dds\n\n[GENERAL]\nCustomCarHUDs = 1 // 0 - disable\nScale = 1.5\n";

    #[test]
    fn typed_reads_use_the_section_and_fall_back_to_the_default() {
        let doc = Doc::parse(SAMPLE);
        assert!(doc.flag("TACHOMETER_GAUGE", "Enabled", false));
        assert_eq!(doc.float("TACHOMETER_GAUGE", "Size", 0.0), 330.0);
        assert_eq!(doc.colour("TACHOMETER_GAUGE", "Color"), Colour(0xFF8E_C8FF));
        assert_eq!(doc.text("TACHOMETER_GAUGE", "ArrowTexture"), "tach-arrow.dds");
        assert_eq!(doc.int("GENERAL", "CustomCarHUDs", 0), 1);
        assert_eq!(doc.float("GENERAL", "Scale", 1.0), 1.5);
        assert_eq!(doc.int("GENERAL", "Missing", 7), 7);
        assert_eq!(doc.text("NOS_GAUGE", "ArrowTexture"), "");
        assert!(!doc.flag("NOS_GAUGE", "Enabled", false));
    }

    #[test]
    fn a_later_line_overrides_an_earlier_one_and_bad_text_is_the_default() {
        let doc = Doc::parse("[A]\nSize = 10\nSize = 20\nBroken = nope\n");
        assert_eq!(doc.float("A", "Size", 0.0), 20.0);
        assert_eq!(doc.float("A", "Broken", 3.0), 3.0);
        assert_eq!(doc.colour("A", "Broken"), Colour::NONE);
    }
}
