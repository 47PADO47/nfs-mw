//! The reader for the Extra Options settings file: `[Section]` headers, `Key = value` lines and
//! `//` comments. Pure text in, pairs out; no file access.

/// One `Section.Key = value` line, with the section and key as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub section: String,
    pub key: String,
    pub value: String,
}

/// Every key/value line of `text`, in order. Lines outside a section, lines without `=` and
/// empty keys are skipped.
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn switches_accept_numbers_and_words() {
        assert_eq!(switch("1"), Some(true));
        assert_eq!(switch("False"), Some(false));
        assert_eq!(switch("2"), None);
    }
}
