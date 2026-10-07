//! Section numbers: `letter × 100 + n`, with `A` = 1 … `Z` = 26
//! (`docs/specs/visible-sections.md`).

/// Interprets section numbers with a track's far-section offset (`LODOffset`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Numbering {
    pub lod_offset: i16,
}

impl Numbering {
    /// The section's letter, if its number has one.
    pub fn letter(section: i16) -> Option<char> {
        let index = u8::try_from(section / 100).ok().filter(|i| (1..=26).contains(i))?;
        Some(char::from(b'A' + index - 1))
    }

    /// The number within the letter.
    pub fn subsection(section: i16) -> i16 {
        section % 100
    }

    /// `A41`, `X0`, …; the plain number when it has no letter.
    pub fn name(section: i16) -> String {
        match Self::letter(section) {
            Some(letter) => format!("{letter}{}", Self::subsection(section)),
            None => section.to_string(),
        }
    }

    /// Parse `A41` into 141.
    pub fn parse(name: &str) -> Option<i16> {
        let mut chars = name.chars();
        let letter = chars.next()?.to_ascii_uppercase();
        let sub: i16 = chars.as_str().parse().ok()?;
        (letter.is_ascii_uppercase() && (0..100).contains(&sub)).then(|| (letter as i16 - 'A' as i16 + 1) * 100 + sub)
    }

    /// A map section (letters `A` to `T`), as opposed to the shared sets.
    pub fn is_regular(section: i16) -> bool {
        Self::letter(section).is_some_and(|l| l < 'U')
    }

    /// A drivable section: regular, with `1 <= n < LODOffset`.
    pub fn is_drivable(&self, section: i16) -> bool {
        Self::is_regular(section) && (1..self.lod_offset).contains(&Self::subsection(section))
    }

    /// A far section: `LODOffset <= n < 2 × LODOffset`.
    pub fn is_far(&self, section: i16) -> bool {
        (self.lod_offset..self.lod_offset * 2).contains(&Self::subsection(section))
    }

    /// The far counterpart of a drivable section.
    pub fn far_of(&self, drivable: i16) -> i16 {
        drivable + self.lod_offset
    }

    /// Texture sections (`Y`, `W`) stay loaded once loaded.
    pub fn is_texture(section: i16) -> bool {
        matches!(Self::letter(section), Some('Y' | 'W'))
    }

    /// Library (model) sections (`X`, `U`) stay loaded once loaded.
    pub fn is_library(section: i16) -> bool {
        matches!(Self::letter(section), Some('X' | 'U'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        let n = Numbering { lod_offset: 40 };
        assert_eq!(Numbering::name(141), "A41");
        assert_eq!(Numbering::name(2400), "X0");
        assert_eq!(Numbering::parse("A41"), Some(141));
        assert_eq!(Numbering::parse("z0"), Some(2600));
        assert_eq!(Numbering::parse("A"), None);
        assert!(n.is_drivable(101) && !n.is_drivable(141) && !n.is_drivable(100) && !n.is_drivable(2101));
        assert!(n.is_far(141) && !n.is_far(190) && !n.is_far(101));
        assert_eq!(n.far_of(101), 141);
        assert!(Numbering::is_regular(2099) && !Numbering::is_regular(2101));
        assert!(Numbering::is_texture(2500) && Numbering::is_library(2400) && !Numbering::is_library(2200));
    }
}
