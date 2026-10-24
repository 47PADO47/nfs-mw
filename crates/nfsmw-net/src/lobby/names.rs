//! Display names that cannot be mistaken for each other (`security-gameplay.md` section 1.1, rule 7).
//!
//! Two names are the same name when their confusable skeletons are (UTS #39: the Cyrillic `а` and the Latin `a`, a
//! capital `I` and a small `l`) once the case is folded. A joiner whose name is the same as a member's gets a number
//! after it, so nobody can sit in the roster as a copy of someone else.

use std::collections::HashSet;

use unicode_security::confusable_detection::skeleton;

use crate::message::Name;

/// The longest suffix tried: ` 2` to ` 99`.
const MAX_SUFFIX: u32 = 99;

/// What two names must differ in to be told apart.
fn key(name: &str) -> String {
    skeleton(name).collect::<String>().to_lowercase()
}

/// `wanted` when no name of `taken` is the same name, else `wanted` cut short and numbered until none is. The name keeps
/// to [`Name`]'s length. `wanted` itself, when every number is taken (nobody has 98 copies of a name).
pub(super) fn unique<'a>(wanted: &Name, taken: impl Iterator<Item = &'a str>) -> Name {
    let keys: HashSet<String> = taken.map(key).collect();
    if !keys.contains(&key(wanted.as_str())) {
        return wanted.clone();
    }
    for number in 2..=MAX_SUFFIX {
        let suffix = format!(" {number}");
        let room = 24usize.saturating_sub(suffix.chars().count());
        let base: String = wanted.as_str().chars().take(room).collect();
        let Ok(candidate) = Name::new(&format!("{}{suffix}", base.trim_end())) else { continue };
        if !keys.contains(&key(candidate.as_str())) {
            return candidate;
        }
    }
    wanted.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(text: &str) -> Name {
        Name::new(text).unwrap()
    }

    #[test]
    fn a_name_nobody_has_is_kept() {
        assert_eq!(unique(&name("Ann"), ["Bob", "Cy"].into_iter()), name("Ann"));
        assert_eq!(unique(&name("Ann"), std::iter::empty()), name("Ann"));
    }

    #[test]
    fn a_copy_of_a_name_is_numbered_also_when_it_only_looks_the_same() {
        assert_eq!(unique(&name("Ann"), ["Ann"].into_iter()), name("Ann 2"));
        assert_eq!(unique(&name("ANN"), ["Ann"].into_iter()), name("ANN 2"), "case does not tell names apart");
        assert_eq!(unique(&name("\u{430}nn"), ["ann"].into_iter()), name("\u{430}nn 2"), "a Cyrillic a is an a");
        assert_eq!(unique(&name("Ann"), ["Ann", "Ann 2"].into_iter()), name("Ann 3"));
    }

    #[test]
    fn a_long_name_is_cut_to_make_room_for_its_number() {
        let long = "x".repeat(24);
        let numbered = unique(&name(&long), [long.as_str()].into_iter());
        assert_eq!(numbered.as_str().chars().count(), 24);
        assert!(numbered.as_str().ends_with(" 2"));
    }
}
