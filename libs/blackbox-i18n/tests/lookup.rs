//! The lookup order of `Strings`: locale first, then layer priority; packs before tables; the key as the last resort.

use blackbox_i18n::{Args, Catalog, GameLayer, Locale, Source, Strings};
use blackbox_text::upper_label_hash;

fn l(tag: &str) -> Locale {
    Locale::parse(tag).unwrap()
}

fn source(namespace: &str, tag: &str, origin: &str, text: &str) -> Source {
    Source {
        namespace: namespace.to_owned(),
        locale: l(tag),
        origin: origin.to_owned(),
        files: vec![("test.ftl".to_owned(), text.to_owned())],
    }
}

fn pack(tag: &str, text: &str) -> GameLayer {
    let (layer, problems) = GameLayer::pack(l(tag), "pack", &[("game.ftl".to_owned(), text.to_owned())]);
    assert!(problems.is_empty(), "{problems:?}");
    layer
}

const CORE_EN: &str = "
back = Back
votes = { $n ->
    [one] { $n } vote
   *[other] { $n } votes
}
place = { NUMBER($n, type: \"ordinal\") ->
    [one] { $n }st
    [two] { $n }nd
    [few] { $n }rd
   *[other] { $n }th
}
only-english = English only
";

const CORE_DE: &str = "
back = Zurück
votes = { $n ->
    [one] { $n } Stimme
   *[other] { $n } Stimmen
}
";

fn catalog(sources: &[Source], pseudo: bool) -> Catalog {
    let (catalog, problems) = Catalog::build(sources, pseudo);
    assert!(problems.is_empty(), "{problems:?}");
    catalog
}

#[test]
fn messages_follow_the_chain_then_fall_back_to_the_id() {
    let catalog = catalog(&[source("core", "de", "built-in", CORE_DE), source("core", "en", "built-in", CORE_EN)], false);
    let strings = Strings::new(l("de-AT"), Vec::new(), catalog);
    assert_eq!(strings.text("core", "back", &Args::new()), "Zurück");
    assert_eq!(strings.text("core", "only-english", &Args::new()), "English only");
    assert_eq!(strings.text("core", "no-such-message", &Args::new()), "no-such-message");
    assert_eq!(strings.try_text("core", "no-such-message", &Args::new()), None);
}

#[test]
fn plurals_and_ordinals_use_the_answering_language() {
    let catalog = catalog(&[source("core", "de", "built-in", CORE_DE), source("core", "en", "built-in", CORE_EN)], false);
    let de = Strings::new(l("de"), Vec::new(), catalog);
    assert_eq!(de.text("core", "votes", &Args::new().with("n", 1)), "1 Stimme");
    assert_eq!(de.text("core", "votes", &Args::new().with("n", 3)), "3 Stimmen");
    assert_eq!(de.text("core", "place", &Args::new().with("n", 2)), "2nd");
    assert_eq!(de.text("core", "place", &Args::new().with("n", 13)), "13th");
    assert_eq!(de.text("core", "place", &Args::new().with("n", 23)), "23rd");
}

#[test]
fn a_higher_source_overrides_a_lower_one_in_the_same_locale_only() {
    let override_de = source("core", "de", "pack", "back = Zurueck");
    let catalog = catalog(
        &[override_de, source("core", "de", "built-in", CORE_DE), source("core", "en", "built-in", CORE_EN)],
        false,
    );
    let strings = Strings::new(l("de"), Vec::new(), catalog);
    assert_eq!(strings.text("core", "back", &Args::new()), "Zurueck");
    assert_eq!(strings.text("core", "votes", &Args::new().with("n", 2)), "2 Stimmen");
}

#[test]
fn namespaces_are_separate() {
    let catalog = catalog(&[source("mod:hello", "en", "hello", "back = Leave"), source("core", "en", "built-in", CORE_EN)], false);
    let strings = Strings::new(l("en"), Vec::new(), catalog);
    assert_eq!(strings.text("mod:hello", "back", &Args::new()), "Leave");
    assert_eq!(strings.text("core", "back", &Args::new()), "Back");
    assert_eq!(strings.try_text("mod:hello", "votes", &Args::new()), None);
}

#[test]
fn game_labels_try_each_locale_and_packs_win_within_one() {
    let de_pack = pack("de", "CARNAME_TEST = Testwagen\nPAUSED = Pause (Paket)");
    let en_pack = pack("en", "PAUSED = Paused\ncarname_only_english = Only English");
    let strings = Strings::new(l("de"), vec![de_pack, en_pack], Catalog::default());
    assert_eq!(strings.game(upper_label_hash("CARNAME_TEST")).as_deref(), Some("Testwagen"));
    assert_eq!(strings.game(upper_label_hash("PAUSED")).as_deref(), Some("Pause (Paket)"));
    assert_eq!(strings.game(upper_label_hash("CARNAME_ONLY_ENGLISH")).as_deref(), Some("Only English"));
    assert_eq!(strings.game(upper_label_hash("MISSING")), None);
}

#[test]
fn game_pack_values_keep_printf_places_and_literal_braces() {
    let layer = pack("en", "RACE_WON = %s won { \"{\" }%d{ \"}\" }");
    assert_eq!(layer.get(upper_label_hash("RACE_WON")).as_deref(), Some("%s won {%d}"));
}

#[test]
fn a_game_pack_rejects_variables() {
    let (_, problems) = GameLayer::pack(l("en"), "pack", &[("game.ftl".to_owned(), "LABEL = { $n } cars".to_owned())]);
    assert_eq!(problems.len(), 1);
}

#[test]
fn the_pseudo_locale_wraps_english_messages_only() {
    let catalog = catalog(&[source("core", "en", "built-in", CORE_EN)], true);
    let strings = Strings::new(Locale::pseudo(), vec![pack("en", "PAUSED = Paused")], catalog);
    assert_eq!(strings.text("core", "back", &Args::new()), "[Báçk~~]");
    assert_eq!(strings.text("core", "votes", &Args::new().with("n", 2)), "[2 vótés~~~]");
    assert_eq!(strings.game(upper_label_hash("PAUSED")).as_deref(), Some("Paused"));
}

#[test]
fn a_broken_file_reports_and_keeps_its_good_messages() {
    let (catalog, problems) = Catalog::build(&[source("core", "en", "built-in", "good = Good\nbad = { $\nalso = Also")], false);
    assert!(!problems.is_empty());
    let strings = Strings::new(l("en"), Vec::new(), catalog);
    assert_eq!(strings.text("core", "good", &Args::new()), "Good");
    assert_eq!(strings.text("core", "also", &Args::new()), "Also");
}

#[test]
fn locales_lists_each_once_in_priority_order() {
    let catalog = catalog(
        &[source("core", "it", "pack", "back = Indietro"), source("core", "en", "built-in", CORE_EN), source("core", "it", "x", "a = b")],
        false,
    );
    assert_eq!(catalog.locales("core"), vec![l("it"), l("en")]);
    assert!(catalog.has("core", "back", &l("it")));
    assert!(!catalog.has("core", "votes", &l("it")));
}
