//! The value types of a skin ini, and how their text is read: a colour as `0xAARRGGBB`, an integer that is hexadecimal
//! after `0x`, a float. Trailing text after a number is ignored, as the C library parsers the plugin uses ignore it.

/// A 32-bit colour, `0xAARRGGBB`. An absent colour is 0: fully transparent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Colour(pub u32);

impl Colour {
    /// The colour of a key that is not set.
    pub const NONE: Colour = Colour(0);

    pub fn alpha(self) -> u8 {
        (self.0 >> 24) as u8
    }

    /// The colour as `[r, g, b, a]` bytes.
    pub fn rgba(self) -> [u8; 4] {
        [(self.0 >> 16) as u8, (self.0 >> 8) as u8, self.0 as u8, (self.0 >> 24) as u8]
    }

    /// The same colour with its alpha byte replaced.
    pub fn with_alpha(self, alpha: u8) -> Colour {
        Colour((self.0 & 0x00ff_ffff) | (u32::from(alpha) << 24))
    }

    /// The same colour with its alpha byte multiplied by `factor` (0 to 1) and cut off to a whole byte.
    pub fn faded(self, factor: f32) -> Colour {
        self.with_alpha((f32::from(self.alpha()) * factor).clamp(0.0, 255.0) as u8)
    }

    /// Whether the key was unset (or set to 0).
    pub fn is_none(self) -> bool {
        self.0 == 0
    }
}

/// An integer as the plugin reads it: optional sign, then hexadecimal digits after `0x` or decimal digits. Text after
/// the digits is ignored. `None` when there are no digits.
pub fn parse_int(text: &str) -> Option<i64> {
    let text = text.trim_start();
    let (negative, rest) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (radix, digits) = match rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        Some(digits) => (16_u32, digits),
        None => (10_u32, rest),
    };
    let end = digits.find(|c: char| !c.is_digit(radix)).unwrap_or(digits.len());
    if end == 0 {
        return None;
    }
    let value = i64::from_str_radix(&digits[..end], radix).ok()?;
    Some(match negative {
        true => -value,
        false => value,
    })
}

/// A float: the longest prefix of the text that is a finite number. `None` when there is none.
pub fn parse_float(text: &str) -> Option<f32> {
    let text = text.trim();
    let mut best = None;
    for end in text.char_indices().map(|(at, c)| at + c.len_utf8()) {
        if let Ok(value) = text[..end].parse::<f32>()
            && value.is_finite()
        {
            best = Some(value);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_read_hex_after_0x_and_ignore_trailing_text() {
        assert_eq!(parse_int("42"), Some(42));
        assert_eq!(parse_int(" -7 // note"), Some(-7));
        assert_eq!(parse_int("0xFFFF0000"), Some(0xFFFF_0000));
        assert_eq!(parse_int("0Xff"), Some(255));
        assert_eq!(parse_int("1 // 0 - disable"), Some(1));
        assert_eq!(parse_int("0"), Some(0));
        assert_eq!(parse_int("abc"), None);
        assert_eq!(parse_int(""), None);
        assert_eq!(parse_int("0x"), None);
    }

    #[test]
    fn floats_take_the_longest_numeric_prefix() {
        assert_eq!(parse_float("330"), Some(330.0));
        assert_eq!(parse_float("-0.5"), Some(-0.5));
        assert_eq!(parse_float("1.0 // scale"), Some(1.0));
        assert_eq!(parse_float("1e"), Some(1.0));
        assert_eq!(parse_float("x"), None);
        assert_eq!(parse_float(""), None);
        assert_eq!(parse_float("inf"), None);
    }

    #[test]
    fn colours_split_into_channels_and_fade_by_alpha() {
        let colour = Colour(0xAA0C_1A30);
        assert_eq!(colour.rgba(), [0x0C, 0x1A, 0x30, 0xAA]);
        assert_eq!(colour.alpha(), 0xAA);
        assert_eq!(colour.with_alpha(0x10), Colour(0x100C_1A30));
        assert_eq!(Colour(0xFF11_2233).faded(0.5).alpha(), 127);
        assert_eq!(Colour(0xFF11_2233).faded(2.0).alpha(), 255);
        assert!(Colour::NONE.is_none());
        assert!(!colour.is_none());
    }
}
