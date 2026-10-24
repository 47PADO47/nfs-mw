use blackbox_i18n::{Args, Locale};

use super::CORE;
use super::build::build;
use super::select::available;

#[test]
fn the_built_in_catalogue_answers_without_an_install() {
    let active = build(None, Locale::english());
    assert_eq!(active.strings.text(CORE, "group-separator", &Args::new()), ",");
    assert!(active.table.is_none());
    assert!(active.is_english());
    assert!(!active.has_voice());
}

#[test]
fn every_built_in_file_parses() {
    let (_, problems) = blackbox_i18n::Catalog::build(&super::catalog::built_in(), false);
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_language_without_a_catalogue_falls_back_to_english() {
    let active = build(None, Locale::parse("de").unwrap());
    assert_eq!(active.strings.text(CORE, "format-percent", &Args::new().with("n", 5)), "5%");
    assert!(!active.is_english());
}

#[test]
fn the_pseudo_locale_brackets_catalogue_text() {
    let active = build(None, Locale::pseudo());
    let text = active.strings.text(CORE, "language-name", &Args::new());
    assert!(text.starts_with("[Éñglísh"), "{text}");
}

#[test]
fn english_is_on_offer_without_an_install() {
    let choices = available(None);
    assert_eq!(choices.first().map(|c| c.name.as_str()), Some("English"));
}

#[test]
fn the_macro_formats_variables() {
    assert_eq!(t!("format-percent", n = 80), "80%");
}
