//! Keep the rewrite's translation catalogue and the code in step (`docs/specs/localization.md` section 8):
//!
//! - every message id the code asks for (`t!("id"`, `Text::key("id"`, `Text::Key("id"`, `Title::Key("id"`,
//!   `i18n::text("id"`) is in the English catalogue;
//! - every English message is named somewhere in the code as a string literal;
//! - another language's catalogue has no message English lacks, and uses no variable the English message does not have.
//!
//! The catalogue is `crates/nfsmw/i18n/<locale>/*.ftl`. Messages are read line by line: a message starts at a line that
//! begins with its id and `=`, and goes on over the indented lines after it, as Fluent writes them.

use std::collections::{BTreeMap, BTreeSet};

use crate::git;

const CATALOGUE: &str = "crates/nfsmw/i18n/";
const CODE: &str = "crates/nfsmw/src/";
/// How the code asks for a message: the id follows one of these.
const CALLS: &[&str] = &["t!(\"", "Text::key(\"", "Text::Key(\"", "Title::Key(\"", "i18n::text(\""];

/// A locale's messages: id to the variables its text uses.
type Messages = BTreeMap<String, BTreeSet<String>>;

/// Checks the working tree: every tracked file and every untracked one `git add .` could pick up.
pub fn run() -> Result<bool, String> {
    let files = git::files_to_check(false)?;
    let mut catalogue: BTreeMap<String, Messages> = BTreeMap::new();
    let mut code = String::new();
    for path in &files {
        let wanted = (path.starts_with(CATALOGUE) && path.ends_with(".ftl")) || (path.starts_with(CODE) && path.ends_with(".rs"));
        if !wanted {
            continue;
        }
        let Some(bytes) = git::read(path, false)? else { continue };
        let text = String::from_utf8_lossy(&bytes);
        if let Some(rest) = path.strip_prefix(CATALOGUE).filter(|_| path.ends_with(".ftl")) {
            let locale = rest.split('/').next().unwrap_or_default().to_owned();
            catalogue.entry(locale).or_default().extend(messages(&text));
            continue;
        }
        if path.starts_with(CODE) && path.ends_with(".rs") {
            code.push_str(&text);
            code.push('\n');
        }
    }
    let problems = problems(&catalogue, &code);
    for problem in &problems {
        eprintln!("i18n-check: {problem}");
    }
    if problems.is_empty() {
        let count = catalogue.get("en").map_or(0, BTreeMap::len);
        eprintln!("i18n-check: {count} English message(s) OK");
    }
    Ok(problems.is_empty())
}

fn problems(catalogue: &BTreeMap<String, Messages>, code: &str) -> Vec<String> {
    let empty = Messages::new();
    let english = catalogue.get("en").unwrap_or(&empty);
    let mut out = Vec::new();
    for id in asked(code) {
        if !english.contains_key(&id) {
            out.push(format!("the code asks for `{id}`, which crates/nfsmw/i18n/en/ does not have"));
        }
    }
    for id in english.keys() {
        if !code.contains(&format!("\"{id}\"")) {
            out.push(format!("en: `{id}` is not used by the code"));
        }
    }
    for (locale, messages) in catalogue.iter().filter(|(locale, _)| locale.as_str() != "en") {
        for (id, variables) in messages {
            let Some(known) = english.get(id) else {
                out.push(format!("{locale}: `{id}` is not an English message"));
                continue;
            };
            for variable in variables.difference(known) {
                out.push(format!("{locale}: `{id}` uses ${variable}, which the English message does not have"));
            }
        }
    }
    out
}

/// The message ids the code asks for.
fn asked(code: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for call in CALLS {
        for (at, _) in code.match_indices(call) {
            let rest = &code[at + call.len()..];
            let Some(end) = rest.find('"') else { continue };
            out.insert(rest[..end].to_owned());
        }
    }
    out
}

/// The messages of one `.ftl` file. Terms (`-name`), comments and attributes are not messages.
fn messages(text: &str) -> Messages {
    let mut out = Messages::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        if line.starts_with([' ', '\t']) {
            if let Some(id) = &current {
                out.entry(id.clone()).or_default().extend(variables(line));
            }
            continue;
        }
        current = None;
        let Some((id, value)) = line.split_once('=') else { continue };
        let id = id.trim();
        if !id.starts_with(|c: char| c.is_ascii_alphabetic()) || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            continue;
        }
        out.entry(id.to_owned()).or_default().extend(variables(value));
        current = Some(id.to_owned());
    }
    out
}

/// The `$name` variables a piece of a message uses.
fn variables(text: &str) -> Vec<String> {
    text.match_indices('$')
        .map(|(at, _)| {
            text[at + 1..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect::<String>()
        })
        .filter(|name| !name.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EN: &str = "## Group\nback = Back\nvotes = { $n ->\n    [one] { $n } vote\n   *[other] { $n } votes\n}\n-term = x\n";

    fn catalogue(pairs: &[(&str, &str)]) -> BTreeMap<String, Messages> {
        pairs.iter().map(|(locale, text)| ((*locale).to_owned(), messages(text))).collect()
    }

    #[test]
    fn reads_messages_and_their_variables() {
        let read = messages(EN);
        assert_eq!(read.keys().collect::<Vec<_>>(), ["back", "votes"]);
        assert_eq!(read["votes"].iter().collect::<Vec<_>>(), ["n"]);
    }

    #[test]
    fn finds_what_the_code_asks_for() {
        let code = r#"t!("back"); Text::key("votes"); t!("lan-votes", n = 3); let x = "other";"#;
        assert_eq!(asked(code).into_iter().collect::<Vec<_>>(), ["back", "lan-votes", "votes"]);
    }

    #[test]
    fn reports_missing_unused_and_foreign_messages() {
        let found = problems(
            &catalogue(&[("en", EN), ("de", "back = Zurück\nvotes = { $count }\nextra = x")]),
            r#"t!("back"); t!("missing"); match k { _ => "votes" }"#,
        );
        assert!(found.iter().any(|p| p.contains("`missing`")));
        assert!(found.iter().any(|p| p.contains("de: `extra`")));
        assert!(found.iter().any(|p| p.contains("$count")));
        assert!(!found.iter().any(|p| p.contains("en: `votes` is not used")));
        assert_eq!(found.len(), 3, "{found:?}");
    }
}
