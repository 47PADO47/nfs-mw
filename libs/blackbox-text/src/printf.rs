//! The printf places of language strings: `%s`, `%d`, `%$d`, `%$.0f`, `%02d` and the like, filled by the caller.

/// `template` with its printf places filled in order from `args`. A place past the last argument is left empty, `%%` is a
/// percent sign, and a `%` that starts no place is kept as it is.
///
/// A place is `%`, then any of `$ 0-9 . - + #` and a space, then one of `s d i u f x c`. The flags are not applied: the
/// argument is already the text to show.
pub fn fill(template: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut next = args.iter();
    let mut rest = template;
    while let Some(at) = rest.find('%') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        if let Some(tail) = after.strip_prefix('%') {
            out.push('%');
            rest = tail;
            continue;
        }
        let flags = after.bytes().take_while(|b| matches!(b, b'$' | b'0'..=b'9' | b'.' | b'-' | b'+' | b'#' | b' ')).count();
        let Some(kind) = after[flags..].chars().next().filter(|c| matches!(c, 's' | 'd' | 'i' | 'u' | 'f' | 'x' | 'c')) else {
            out.push('%');
            rest = after;
            continue;
        };
        out.push_str(next.next().copied().unwrap_or(""));
        rest = &after[flags + kind.len_utf8()..];
    }
    out.push_str(rest);
    out
}

/// How many printf places `template` has.
pub fn places(template: &str) -> usize {
    let probe: Vec<&str> = vec!["\u{1}"; template.len()];
    fill(template, &probe).matches('\u{1}').count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_places_in_order() {
        assert_eq!(fill("%s won %d races", &["Razor", "3"]), "Razor won 3 races");
        assert_eq!(fill("Bounty: %$d", &["1,500"]), "Bounty: 1,500");
        assert_eq!(fill("%$.0f sec", &["12"]), "12 sec");
        assert_eq!(fill("Lap %02d", &["7"]), "Lap 7");
    }

    #[test]
    fn missing_arguments_and_stray_percents() {
        assert_eq!(fill("%s and %s", &["one"]), "one and ");
        assert_eq!(fill("100%% done", &[]), "100% done");
        assert_eq!(fill("50% off", &[]), "50% off");
        assert_eq!(fill("no places", &["unused"]), "no places");
    }

    #[test]
    fn counts_places() {
        assert_eq!(places("%s won %d of %$d"), 3);
        assert_eq!(places("100%%"), 0);
    }
}
