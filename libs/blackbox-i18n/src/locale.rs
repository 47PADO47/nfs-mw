//! BCP 47 locales, their fallback chain and the choice among the locales on offer.

use std::fmt;

use unic_langid::LanguageIdentifier;

/// The tag of the pseudo-locale made from English for testing ([`crate::pseudo`]).
const PSEUDO: &str = "en-XA";

/// A language tag such as `en`, `de` or `pt-BR`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Locale(LanguageIdentifier);

impl Locale {
    /// Reads a tag. Also takes the POSIX spelling of the system (`pt_BR.UTF-8`, `de_DE@euro`). `None` for text that is no tag.
    pub fn parse(tag: &str) -> Option<Self> {
        let tag = tag.split(['.', '@']).next().unwrap_or("").trim().replace('_', "-");
        if tag.is_empty() {
            return None;
        }
        tag.parse::<LanguageIdentifier>().ok().map(Self)
    }

    /// English, the last locale of every chain.
    pub fn english() -> Self {
        Self::parse("en").expect("en is a tag")
    }

    /// The pseudo-locale `en-XA`.
    pub fn pseudo() -> Self {
        Self::parse(PSEUDO).expect("en-XA is a tag")
    }

    pub fn is_pseudo(&self) -> bool {
        *self == Self::pseudo()
    }

    /// The language subtag alone (`pt` for `pt-BR`).
    pub fn language(&self) -> &str {
        self.0.language.as_str()
    }

    /// The identifier Fluent takes.
    pub fn id(&self) -> &LanguageIdentifier {
        &self.0
    }

    /// The locale with only its language (`pt` for `pt-BR`); `None` for a locale that is only a language.
    pub fn parent(&self) -> Option<Self> {
        let base = LanguageIdentifier::from_parts(self.0.language, None, None, &[]);
        if base == self.0 {
            return None;
        }
        Some(Self(base))
    }

    /// The locales to try in order: this one, its parent, then English.
    pub fn chain(&self) -> Vec<Self> {
        let mut chain = vec![self.clone()];
        chain.extend(self.parent());
        let english = Self::english();
        if !chain.contains(&english) {
            chain.push(english);
        }
        chain
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The locale of `available` that best serves `wanted`: the same tag, else the bare language, else another locale of the
/// same language. `None` when no available locale shares the language.
pub fn negotiate(wanted: &Locale, available: &[Locale]) -> Option<Locale> {
    if available.contains(wanted) {
        return Some(wanted.clone());
    }
    if let Some(parent) = wanted.parent().filter(|parent| available.contains(parent)) {
        return Some(parent);
    }
    available.iter().find(|locale| locale.language() == wanted.language()).cloned()
}

/// The system's preferred locales, most preferred first. Empty when the system does not say.
pub fn system_locales() -> Vec<Locale> {
    sys_locale::get_locales().filter_map(|tag| Locale::parse(&tag)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(tag: &str) -> Locale {
        Locale::parse(tag).unwrap()
    }

    #[test]
    fn parses_tags_and_posix_names() {
        assert_eq!(l("pt_BR.UTF-8"), l("pt-BR"));
        assert_eq!(l("de_DE@euro"), l("de-DE"));
        assert_eq!(Locale::parse(""), None);
        assert_eq!(Locale::parse("not a tag!"), None);
    }

    #[test]
    fn chain_ends_in_english() {
        assert_eq!(l("pt-BR").chain(), vec![l("pt-BR"), l("pt"), l("en")]);
        assert_eq!(l("de").chain(), vec![l("de"), l("en")]);
        assert_eq!(l("en-GB").chain(), vec![l("en-GB"), l("en")]);
        assert_eq!(l("en").chain(), vec![l("en")]);
        assert_eq!(Locale::pseudo().chain(), vec![Locale::pseudo(), l("en")]);
    }

    #[test]
    fn negotiation_prefers_the_closest_locale() {
        let available = [l("en"), l("de"), l("pt-BR")];
        assert_eq!(negotiate(&l("de-AT"), &available), Some(l("de")));
        assert_eq!(negotiate(&l("pt"), &available), Some(l("pt-BR")));
        assert_eq!(negotiate(&l("pt-BR"), &available), Some(l("pt-BR")));
        assert_eq!(negotiate(&l("fr"), &available), None);
    }
}
