//! The pseudo-locale `en-XA`: English with every vowel and some consonants accented, a third longer and in brackets, so
//! that text which does not come from a catalogue stands out on screen and long translations can be tried.

use std::borrow::Cow;

/// The accented spelling of a piece of a message (Fluent calls it for the text between variables).
pub fn transform(text: &str) -> Cow<'_, str> {
    Cow::Owned(text.chars().map(accent).collect())
}

/// A formatted message as the pseudo-locale shows it: in brackets, padded by a third of its length.
pub fn wrap(text: &str) -> String {
    let pad = text.chars().count().div_ceil(3);
    format!("[{text}{}]", "~".repeat(pad))
}

/// Latin-1 letters only, so the game's own fonts can still draw them.
fn accent(c: char) -> char {
    match c {
        'a' => 'á',
        'e' => 'é',
        'i' => 'í',
        'o' => 'ó',
        'u' => 'ú',
        'y' => 'ý',
        'n' => 'ñ',
        'c' => 'ç',
        'A' => 'Á',
        'E' => 'É',
        'I' => 'Í',
        'O' => 'Ó',
        'U' => 'Ú',
        'Y' => 'Ý',
        'N' => 'Ñ',
        'C' => 'Ç',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accents_and_pads() {
        assert_eq!(transform("Race on"), "Rácé óñ");
        assert_eq!(wrap("Back"), "[Back~~]");
    }
}
