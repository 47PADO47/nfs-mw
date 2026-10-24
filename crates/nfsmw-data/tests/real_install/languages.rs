//! The install's language tables (`docs/specs/localization.md` section 3).

use nfsmw_data::languages;

use crate::install;

#[test]
fn english_is_listed_first_and_reads() {
    let Some(dir) = install() else { return };
    let found = languages::installed(&dir);
    let english = found.first().expect("at least English");
    assert_eq!(english.tag, "en");
    assert_eq!(english.game.map(|g| g.id), Some(0));
    assert!(found.iter().all(|l| !l.file.to_ascii_lowercase().contains("labels")));
    let table = languages::table(&dir, &english.file).expect("English.bin");
    assert!(table.len() > 4000, "{} strings", table.len());
}

#[test]
fn the_registry_names_a_known_language() {
    if install().is_none() {
        return;
    }
    let Some(tag) = languages::registry_language() else { return };
    assert!(languages::by_tag(tag).is_some(), "{tag}");
}
