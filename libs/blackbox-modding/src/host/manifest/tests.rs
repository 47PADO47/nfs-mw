//! Manifest parsing and validation: each reject reason of ADR 0005 section 3 that a manifest alone can show.

use super::*;

const MINIMAL: &str = r#"
[mod]
id = "example.hello"
name = "Hello"
version = "0.1.0"
[api]
blackbox_modding = "0.1"
[games]
supported = ["*"]
"#;

fn with(extra: &str) -> String {
    format!("{MINIMAL}\n{extra}")
}

#[test]
fn a_minimal_manifest_parses_with_defaults() {
    let manifest = parse(MINIMAL).expect("minimal manifest");
    assert_eq!(manifest.id, "example.hello");
    assert_eq!(manifest.name, "Hello");
    assert_eq!(manifest.version, Version::new(0, 1, 0));
    assert_eq!(manifest.api, AbiVersion { major: 0, minor: 1 });
    assert_eq!(manifest.games, GameSupport::Any);
    assert_eq!(manifest.library, Library::default());
    assert!(manifest.required_interfaces.is_empty());
    assert!(manifest.dependencies.is_empty());
    assert_eq!(manifest.priority, 0);
}

#[test]
fn every_optional_field_is_read() {
    let text = with(
        r#"
[library]
windows = "hello.dll"
linux = "libhello.so"
macos = "libhello.dylib"
[requires.interfaces]
"bb.commands" = "1.0"
[optional.interfaces]
"bb.extra" = "1.2"
[dependencies]
"example.base" = ">=1.2, <2"
[load]
after = ["example.other"]
before = ["example.late"]
[content]
priority = 5
"#,
    );
    let text = text.replace("[mod]", "[mod]\nauthors = [\"Someone\"]\nlicense = \"MIT\"");
    let manifest = parse(&text).expect("full manifest");
    assert_eq!(manifest.authors, vec!["Someone".to_owned()]);
    assert_eq!(manifest.license.as_deref(), Some("MIT"));
    assert_eq!(manifest.library.file_for("windows"), Some("hello.dll"));
    assert_eq!(manifest.library.file_for("linux"), Some("libhello.so"));
    assert_eq!(manifest.library.file_for("macos"), Some("libhello.dylib"));
    assert_eq!(manifest.library.file_for("plan9"), None);
    assert_eq!(manifest.required_interfaces[0].id, "bb.commands");
    assert_eq!(manifest.required_interfaces[0].version, InterfaceVersion { major: 1, minor: 0 });
    assert_eq!(manifest.optional_interfaces[0].version, InterfaceVersion { major: 1, minor: 2 });
    assert_eq!(manifest.dependencies[0].id, "example.base");
    assert!(manifest.dependencies[0].requirement.matches(&Version::new(1, 9, 0)));
    assert!(!manifest.dependencies[0].requirement.matches(&Version::new(2, 0, 0)));
    assert_eq!(manifest.load_after, vec!["example.other".to_owned()]);
    assert_eq!(manifest.load_before, vec!["example.late".to_owned()]);
    assert_eq!(manifest.priority, 5);
}

#[test]
fn a_named_game_list_is_kept() {
    let text = MINIMAL.replace(r#"supported = ["*"]"#, r#"supported = ["testgame", "othergame"]"#);
    let manifest = parse(&text).expect("named games");
    assert!(manifest.games.supports("testgame"));
    assert!(!manifest.games.supports("other"));
    assert!(GameSupport::Any.supports("anything"));
}

#[test]
fn unknown_keys_are_ignored_for_forward_compatibility() {
    let text = with("[future]\nthing = 1\n");
    assert!(parse(&text).is_ok());
}

#[test]
fn a_missing_required_section_is_a_parse_error() {
    let text = MINIMAL.replace("[games]\nsupported = [\"*\"]\n", "");
    assert!(matches!(parse(&text), Err(ManifestError::Parse(_))));
}

#[test]
fn invalid_ids_are_rejected() {
    for id in ["", "Example", "a..b", ".hello", "hello.", "a/b", "a b"] {
        let text = MINIMAL.replace("example.hello", id);
        assert_eq!(parse(&text), Err(ManifestError::InvalidId(id.to_owned())), "id {id:?}");
    }
}

#[test]
fn an_id_of_64_bytes_is_the_longest_valid_one() {
    let long = format!("a{}b", "x".repeat(62));
    assert!(is_valid_id(&long));
    assert!(!is_valid_id(&format!("{long}x")));
}

#[test]
fn an_empty_name_is_rejected() {
    let text = MINIMAL.replace("name = \"Hello\"", "name = \"  \"");
    assert_eq!(parse(&text), Err(ManifestError::EmptyName));
}

#[test]
fn a_bad_version_is_rejected() {
    let text = MINIMAL.replace("0.1.0", "1.0");
    assert_eq!(parse(&text), Err(ManifestError::InvalidVersion("1.0".to_owned())));
}

#[test]
fn a_bad_api_version_is_rejected() {
    for value in ["1", "1.0.0", "x.1", "1.-1"] {
        let text = MINIMAL.replace("\"0.1\"", &format!("\"{value}\""));
        assert_eq!(parse(&text), Err(ManifestError::InvalidApiVersion(value.to_owned())), "{value}");
    }
}

#[test]
fn a_bad_interface_version_is_rejected() {
    let text = with("[requires.interfaces]\n\"bb.commands\" = \"one\"\n");
    assert!(matches!(parse(&text), Err(ManifestError::InvalidInterfaceVersion { .. })));
}

#[test]
fn a_bad_dependency_requirement_is_rejected() {
    let text = with("[dependencies]\n\"example.base\" = \"not a req\"\n");
    assert!(matches!(parse(&text), Err(ManifestError::InvalidDependency { .. })));
}

#[test]
fn an_empty_or_malformed_game_list_is_rejected() {
    let empty = MINIMAL.replace(r#"supported = ["*"]"#, "supported = []");
    assert_eq!(parse(&empty), Err(ManifestError::NoGames));
    let star_with_others = MINIMAL.replace(r#"supported = ["*"]"#, r#"supported = ["*", "testgame"]"#);
    assert_eq!(parse(&star_with_others), Err(ManifestError::InvalidGame("*".to_owned())));
    let upper = MINIMAL.replace(r#"supported = ["*"]"#, r#"supported = ["TESTGAME"]"#);
    assert_eq!(parse(&upper), Err(ManifestError::InvalidGame("TESTGAME".to_owned())));
}

#[test]
fn a_library_name_must_be_a_plain_file_of_its_platform() {
    for file in ["../hello.dll", "sub/hello.dll", "C:hello.dll", "hello.so", "hello", ".dll", ".."] {
        let text = with(&format!("[library]\nwindows = \"{file}\"\n"));
        assert!(matches!(parse(&text), Err(ManifestError::InvalidLibrary { key: "windows", .. })), "{file}");
    }
}

#[test]
fn a_self_reference_is_rejected() {
    let text = with("[load]\nafter = [\"example.hello\"]\n");
    assert_eq!(parse(&text), Err(ManifestError::SelfReference("example.hello".to_owned())));
}

#[test]
fn load_hints_are_sorted_and_deduplicated() {
    let text = with("[load]\nafter = [\"b.mod\", \"a.mod\", \"a.mod\"]\n");
    let manifest = parse(&text).expect("hints");
    assert_eq!(manifest.load_after, vec!["a.mod".to_owned(), "b.mod".to_owned()]);
}
