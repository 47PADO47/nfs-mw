//! The chat text policy without a Unicode table crate: a bounded pass over `char`s.
//!
//! Done, in one pass:
//!
//! - control characters (C0, DEL, C1; tab and line breaks included) become a space;
//! - bidirectional controls (`U+200E`, `U+200F`, `U+061C`, `U+202A`-`U+202E`, `U+2066`-`U+2069`) are removed, so a
//!   line cannot reorder the next one;
//! - invisible characters are removed: zero-width space `U+200B`, soft hyphen `U+00AD`, Mongolian vowel separator
//!   `U+180E`, word joiner and invisible operators `U+2060`-`U+2064`, byte order mark `U+FEFF`, combining grapheme
//!   joiner `U+034F`, variation selectors `U+FE00`-`U+FE0F` beyond the first after a character, and noncharacters
//!   (`U+FDD0`-`U+FDEF` and the last two code points of every plane);
//! - zero-width joiner and non-joiner (`U+200C`, `U+200D`) are kept (emoji sequences and Indic scripts need them) but
//!   never two in a row and never first;
//! - combining marks of the generic blocks (`U+0300`-`U+036F`, `U+1AB0`-`U+1AFF`, `U+1DC0`-`U+1DFF`, `U+20D0`-`U+20FF`,
//!   `U+FE20`-`U+FE2F`) are kept up to [`MAX_COMBINING_RUN`] after one base character, which stops "zalgo" text
//!   from drawing over other lines;
//! - runs of whitespace become one space, the ends are trimmed, and the text is cut to [`ChatText`]'s 200 characters.
//!
//! Skipped, because they need the Unicode data tables: NFKC normalisation, confusable and homoglyph folding, counting
//! grapheme clusters instead of `char`s, and the combining marks of script-specific blocks (Hebrew points, Indic
//! signs), which are left alone because those scripts need them. The 200-`char` bound is stricter than a grapheme bound.

use crate::message::ChatText;

/// Most generic combining marks kept after one base character.
pub const MAX_COMBINING_RUN: usize = 2;
/// Most characters of a chat line.
pub const MAX_CHAT_CHARS: usize = 200;

/// `raw` under the policy above, or `None` when nothing printable is left.
pub fn normalise(raw: &str) -> Option<ChatText> {
    let mut out = String::with_capacity(raw.len().min(MAX_CHAT_CHARS * 4));
    let mut chars = 0;
    let mut combining = 0;
    let mut last = Last::Start;
    for c in raw.chars() {
        if chars >= MAX_CHAT_CHARS {
            break;
        }
        let kind = classify(c);
        let keep = match kind {
            Kind::Drop => false,
            Kind::Space => last != Last::Start && last != Last::Space,
            Kind::Joiner => last == Last::Base,
            Kind::Combining => last == Last::Base && combining < MAX_COMBINING_RUN,
            Kind::Selector => last == Last::Base && combining == 0,
            Kind::Base => true,
        };
        if !keep {
            continue;
        }
        combining = match kind {
            Kind::Combining | Kind::Selector => combining + 1,
            _ => 0,
        };
        last = match kind {
            Kind::Space => Last::Space,
            Kind::Joiner => Last::Joiner,
            _ => Last::Base,
        };
        out.push(match kind {
            Kind::Space => ' ',
            _ => c,
        });
        chars += 1;
    }
    ChatText::new(out.trim_end()).ok()
}

/// What came before, for the run rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Last {
    Start,
    Base,
    Space,
    Joiner,
}

/// What the policy does with one character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Drop,
    Space,
    Joiner,
    Combining,
    Selector,
    Base,
}

fn classify(c: char) -> Kind {
    let code = u32::from(c);
    if c.is_control() || c.is_whitespace() {
        return Kind::Space;
    }
    if is_bidi(c) || is_invisible(c) || is_noncharacter(code) {
        return Kind::Drop;
    }
    if matches!(c, '\u{200C}' | '\u{200D}') {
        return Kind::Joiner;
    }
    if matches!(c, '\u{FE00}'..='\u{FE0F}') {
        return Kind::Selector;
    }
    if is_generic_combining(c) {
        return Kind::Combining;
    }
    Kind::Base
}

fn is_bidi(c: char) -> bool {
    matches!(c, '\u{200E}' | '\u{200F}' | '\u{061C}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

fn is_invisible(c: char) -> bool {
    matches!(c, '\u{200B}' | '\u{00AD}' | '\u{180E}' | '\u{2060}'..='\u{2064}' | '\u{FEFF}' | '\u{034F}')
}

fn is_noncharacter(code: u32) -> bool {
    (0xFDD0..=0xFDEF).contains(&code) || code & 0xFFFE == 0xFFFE
}

fn is_generic_combining(c: char) -> bool {
    matches!(
        c,
        '\u{0300}'..='\u{036F}'
            | '\u{1AB0}'..='\u{1AFF}'
            | '\u{1DC0}'..='\u{1DFF}'
            | '\u{20D0}'..='\u{20FF}'
            | '\u{FE20}'..='\u{FE2F}'
    )
}
