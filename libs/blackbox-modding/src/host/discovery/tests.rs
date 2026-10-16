//! Discovery: the policy, the folder layout, the host's offer and duplicate ids.

use super::*;
use crate::host::manifest::InterfaceVersion;
use crate::host::test_support::{Scratch, manifest_text};

fn host() -> HostOffer {
    HostOffer::new("testgame", "test")
}

fn all(roots: &[PathBuf]) -> Discovery {
    discover(roots, &Policy::All, &host())
}

fn rejected_ids(found: &Discovery) -> Vec<(String, RejectReason)> {
    found.rejected.iter().map(|r| (r.name().to_owned(), r.reason.clone())).collect()
}

#[test]
fn policy_off_scans_nothing_and_reports_nothing() {
    let scratch = Scratch::new("off");
    let dir = scratch.folder("a.mod");
    fs::write(dir.join(MANIFEST_FILE), manifest_text("a.mod", "1.0.0", "")).expect("manifest");
    let found = discover(&[scratch.path().to_owned()], &Policy::Off, &host());
    assert_eq!(found, Discovery::default());
}

#[test]
fn a_content_only_mod_is_a_candidate_without_a_library() {
    let scratch = Scratch::new("content");
    let dir = scratch.folder("a");
    fs::write(dir.join(MANIFEST_FILE), manifest_text("a.mod", "1.0.0", "")).expect("manifest");
    let found = all(&[scratch.path().to_owned()]);
    assert!(found.rejected.is_empty());
    assert_eq!(found.candidates.len(), 1);
    assert_eq!(found.candidates[0].manifest.id, "a.mod");
    assert_eq!(found.candidates[0].library, None);
}

#[test]
fn a_missing_mods_folder_is_not_an_error() {
    let scratch = Scratch::new("missing");
    let found = all(&[scratch.path().join("no-such-folder")]);
    assert_eq!(found, Discovery::default());
}

#[test]
fn candidates_come_out_sorted_by_id_whatever_the_folder_names() {
    let scratch = Scratch::new("order");
    for (folder, id) in [("zz", "a.first"), ("aa", "c.last"), ("mm", "b.middle")] {
        let dir = scratch.folder(folder);
        fs::write(dir.join(MANIFEST_FILE), manifest_text(id, "1.0.0", "")).expect("manifest");
    }
    let ids: Vec<String> = all(&[scratch.path().to_owned()]).candidates.into_iter().map(|c| c.manifest.id).collect();
    assert_eq!(ids, ["a.first", "b.middle", "c.last"]);
}

#[test]
fn a_folder_without_a_manifest_is_reported() {
    let scratch = Scratch::new("nomanifest");
    scratch.folder("stray");
    let found = all(&[scratch.path().to_owned()]);
    assert_eq!(rejected_ids(&found), [("stray".to_owned(), RejectReason::ManifestMissing)]);
}

#[test]
fn a_broken_manifest_is_reported_and_the_others_still_load() {
    let scratch = Scratch::new("broken");
    fs::write(scratch.folder("bad").join(MANIFEST_FILE), "[mod\n").expect("manifest");
    let good = scratch.folder("good");
    fs::write(good.join(MANIFEST_FILE), manifest_text("good.mod", "1.0.0", "")).expect("manifest");
    let found = all(&[scratch.path().to_owned()]);
    assert_eq!(found.candidates.len(), 1);
    assert_eq!(found.rejected.len(), 1);
    assert!(matches!(found.rejected[0].reason, RejectReason::Manifest(ManifestError::Parse(_))));
}

#[test]
fn an_allowlist_refuses_every_mod_it_does_not_name() {
    let scratch = Scratch::new("allow");
    for id in ["listed.mod", "other.mod"] {
        let dir = scratch.folder(id);
        fs::write(dir.join(MANIFEST_FILE), manifest_text(id, "1.0.0", "")).expect("manifest");
    }
    let policy = Policy::Allowlist(["listed.mod".to_owned()].into_iter().collect());
    let found = discover(&[scratch.path().to_owned()], &policy, &host());
    assert_eq!(found.candidates.len(), 1);
    assert_eq!(rejected_ids(&found), [("other.mod".to_owned(), RejectReason::NotAllowed)]);
}

#[test]
fn a_future_abi_is_refused_with_a_reason() {
    let scratch = Scratch::new("future");
    let text =
        manifest_text("future.mod", "1.0.0", "").replace("blackbox_modding = \"0.1\"", "blackbox_modding = \"1.0\"");
    fs::write(scratch.folder("future").join(MANIFEST_FILE), text).expect("manifest");
    let found = all(&[scratch.path().to_owned()]);
    assert!(found.candidates.is_empty());
    assert_eq!(
        rejected_ids(&found),
        [("future.mod".to_owned(), RejectReason::AbiNotOffered { module: "1.0".to_owned(), host: "0.1".to_owned() })]
    );
}

#[test]
fn a_game_the_host_does_not_run_is_refused() {
    let scratch = Scratch::new("game");
    let text = manifest_text("game.mod", "1.0.0", "").replace("supported = [\"*\"]", "supported = [\"othergame\"]");
    fs::write(scratch.folder("game").join(MANIFEST_FILE), text).expect("manifest");
    let found = all(&[scratch.path().to_owned()]);
    assert_eq!(
        rejected_ids(&found),
        [("game.mod".to_owned(), RejectReason::GameUnsupported { host: "testgame".to_owned() })]
    );
}

#[test]
fn a_required_interface_must_be_offered_at_a_high_enough_minor() {
    let text = manifest_text("need.mod", "1.0.0", "[requires.interfaces]\n\"bb.commands\" = \"1.2\"\n");
    let scratch = Scratch::new("iface");
    fs::write(scratch.folder("need").join(MANIFEST_FILE), text).expect("manifest");
    let root = [scratch.path().to_owned()];

    let absent = discover(&root, &Policy::All, &host());
    assert_eq!(
        rejected_ids(&absent),
        [("need.mod".to_owned(), RejectReason::InterfaceMissing { id: "bb.commands".to_owned() })]
    );

    let old = HostOffer::new("testgame", "test").with_interface("bb.commands", InterfaceVersion { major: 1, minor: 1 });
    let too_old = discover(&root, &Policy::All, &old);
    assert_eq!(
        rejected_ids(&too_old),
        [(
            "need.mod".to_owned(),
            RejectReason::InterfaceVersion {
                id: "bb.commands".to_owned(),
                required: "1.2".to_owned(),
                offered: "1.1".to_owned(),
            }
        )]
    );

    let enough =
        HostOffer::new("testgame", "test").with_interface("bb.commands", InterfaceVersion { major: 1, minor: 3 });
    assert_eq!(discover(&root, &Policy::All, &enough).candidates.len(), 1);
}

#[test]
fn an_optional_interface_never_rejects_a_mod() {
    let text = manifest_text("opt.mod", "1.0.0", "[optional.interfaces]\n\"bb.extra\" = \"1.0\"\n");
    let scratch = Scratch::new("optional");
    fs::write(scratch.folder("opt").join(MANIFEST_FILE), text).expect("manifest");
    assert_eq!(all(&[scratch.path().to_owned()]).candidates.len(), 1);
}

#[test]
fn a_library_named_for_this_platform_must_be_in_the_folder() {
    let key = match std::env::consts::OS {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "macos",
        _ => return,
    };
    let file = format!("hello{}", std::env::consts::DLL_SUFFIX);
    let text = manifest_text("lib.mod", "1.0.0", &format!("[library]\n{key} = \"{file}\"\n"));
    let scratch = Scratch::new("library");
    let dir = scratch.folder("lib");
    fs::write(dir.join(MANIFEST_FILE), &text).expect("manifest");
    let missing = all(&[scratch.path().to_owned()]);
    assert_eq!(rejected_ids(&missing), [("lib.mod".to_owned(), RejectReason::LibraryMissing { file: file.clone() })]);

    fs::write(dir.join(&file), b"not really a library").expect("library stand-in");
    let present = all(&[scratch.path().to_owned()]);
    assert!(present.rejected.is_empty());
    assert_eq!(present.candidates[0].library, Some(dir.join(&file)));
}

#[test]
fn the_higher_version_of_a_duplicate_id_is_kept() {
    let older = Scratch::new("dup-old");
    let newer = Scratch::new("dup-new");
    fs::write(older.folder("dup").join(MANIFEST_FILE), manifest_text("dup.mod", "1.0.0", "")).expect("manifest");
    fs::write(newer.folder("dup").join(MANIFEST_FILE), manifest_text("dup.mod", "1.2.0", "")).expect("manifest");
    let found = all(&[older.path().to_owned(), newer.path().to_owned()]);
    assert_eq!(found.candidates.len(), 1);
    assert_eq!(found.candidates[0].manifest.version.to_string(), "1.2.0");
    assert_eq!(rejected_ids(&found), [("dup.mod".to_owned(), RejectReason::Duplicate { kept: "1.2.0".to_owned() })]);
}

#[test]
fn an_equal_version_keeps_the_first_root() {
    let first = Scratch::new("eq-first");
    let second = Scratch::new("eq-second");
    fs::write(first.folder("d").join(MANIFEST_FILE), manifest_text("eq.mod", "1.0.0", "")).expect("manifest");
    fs::write(second.folder("d").join(MANIFEST_FILE), manifest_text("eq.mod", "1.0.0", "")).expect("manifest");
    let found = all(&[first.path().to_owned(), second.path().to_owned()]);
    assert_eq!(found.candidates[0].dir, first.path().join("d"));
    assert_eq!(found.rejected[0].dir, second.path().join("d"));
}

#[test]
fn the_game_list_is_checked_against_the_offer() {
    let text = manifest_text("g.mod", "1.0.0", "").replace("supported = [\"*\"]", "supported = [\"testgame\"]");
    let manifest = crate::host::manifest::parse(&text).expect("manifest");
    let other_game = HostOffer::new("othergame", "test");
    assert_eq!(
        check_offer(&manifest, &other_game),
        Err(RejectReason::GameUnsupported { host: "othergame".to_owned() })
    );
    assert_eq!(check_offer(&manifest, &host()), Ok(()));
}
