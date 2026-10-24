//! The `language` setting: the BCP 47 tag of the language the player picked (`docs/specs/localization.md` section 3).
//! Unset, the installer's registry language or the system's is used.

use std::fmt;
use std::str::FromStr;

use blackbox_i18n::Locale;

/// The longest tag kept. Real tags (`en`, `pt-BR`, `zh-Hant-TW`) are far shorter.
const MAX: usize = 23;

/// A language tag, stored inline so the settings stay `Copy`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LanguageTag {
    bytes: [u8; MAX],
    len: u8,
}

impl LanguageTag {
    /// The tag of `locale`; `None` for one longer than any real tag.
    pub fn of(locale: &Locale) -> Option<Self> {
        let text = locale.to_string();
        if text.len() > MAX {
            return None;
        }
        let mut bytes = [0; MAX];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        Some(Self { bytes, len: text.len() as u8 })
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or("en")
    }

    pub fn locale(&self) -> Locale {
        Locale::parse(self.as_str()).unwrap_or_else(Locale::english)
    }
}

impl FromStr for LanguageTag {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        let locale = Locale::parse(s).ok_or_else(|| format!("{s:?} is not a language tag (en, de, pt-BR)"))?;
        Self::of(&locale).ok_or_else(|| format!("{s:?} is too long for a language tag"))
    }
}

impl fmt::Display for LanguageTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for LanguageTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LanguageTag({})", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_tags() {
        assert_eq!("pt_br".parse::<LanguageTag>().unwrap().to_string(), "pt-BR");
        assert_eq!("de".parse::<LanguageTag>().unwrap().locale(), Locale::parse("de").unwrap());
        assert!("not a tag!".parse::<LanguageTag>().is_err());
    }
}
