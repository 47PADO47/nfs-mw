//! `nfsmw strings --match-catalog`: the English messages of the rewrite's catalogue whose text a game label already has.
//! Such a message is better shown as that label, so every language the install has shows it translated
//! (`docs/specs/localization.md` section 8).

use blackbox_i18n::{Args, Catalog, Locale};
use blackbox_text::StringTable;

use super::{CORE, catalog};

/// One line per message with a match: the message id, then each label hash (and name, when `names` knows it) whose
/// English text is the same, ignoring case and surrounding space.
pub fn report(english: &StringTable, names: Option<&StringTable>) -> String {
    let (catalogue, _) = Catalog::build(&catalog::built_in(), false);
    let chain = [Locale::english()];
    let labels: Vec<(u32, String)> =
        english.labels().filter_map(|label| Some((label, normal(&english.get(label)?)))).collect();
    let mut out = String::new();
    for id in message_ids() {
        let Some((text, _)) = catalogue.format(CORE, &id, &Args::new(), &chain) else { continue };
        let wanted = normal(&text);
        if wanted.is_empty() {
            continue;
        }
        let found: Vec<String> = labels
            .iter()
            .filter(|(_, label_text)| *label_text == wanted)
            .map(|(label, _)| match names.and_then(|names| names.get(*label)) {
                Some(name) => format!("{label:#010x} {name}"),
                None => format!("{label:#010x}"),
            })
            .collect();
        if found.is_empty() {
            continue;
        }
        out.push_str(&format!("{id}\t{text}\t{}\n", found.join(", ")));
    }
    out
}

fn normal(text: &str) -> String {
    text.trim().to_lowercase()
}

/// The ids of the English built-in messages, read from the files as `id =` at the start of a line.
fn message_ids() -> Vec<String> {
    let english = Locale::english();
    catalog::built_in()
        .into_iter()
        .filter(|source| source.locale == english)
        .flat_map(|source| source.files.into_iter().map(|(_, text)| text))
        .flat_map(|text| {
            text.lines()
                .filter(|line| line.starts_with(|c: char| c.is_ascii_alphabetic()))
                .filter_map(|line| Some(line.split_once('=')?.0.trim().to_owned()))
                .collect::<Vec<_>>()
        })
        .collect()
}
