//! The runtime without a real library: start-up order, refusals, reload rules and the developer cache. Real loading and
//! dispatch are covered by `tests/load_example.rs`, which builds the `hello` example.

use std::fs;

use super::*;
use crate::host::manifest::MANIFEST_FILE;
use crate::host::test_support::{Scratch, manifest_text};

fn offer() -> HostOffer {
    HostOffer::new("nfsmw", "test")
}

fn write_mod(scratch: &Scratch, folder: &str, text: &str) -> PathBuf {
    let dir = scratch.folder(folder);
    fs::write(dir.join(MANIFEST_FILE), text).expect("manifest");
    dir
}

#[test]
fn content_only_mods_load_in_dependency_order() {
    let scratch = Scratch::new("runtime-order");
    write_mod(&scratch, "b", &manifest_text("b.mod", "1.0.0", "[dependencies]\n\"a.mod\" = \"*\"\n"));
    write_mod(&scratch, "a", &manifest_text("a.mod", "1.2.0", ""));
    let runtime = ModRuntime::start(offer(), &Policy::All, &[scratch.path().to_owned()], RunMode::Normal);
    let ids: Vec<String> = runtime.loaded().into_iter().map(|mod_| mod_.id).collect();
    assert_eq!(ids, ["a.mod", "b.mod"]);
    assert!(runtime.rejected().is_empty());
}

#[test]
fn policy_off_loads_nothing() {
    let scratch = Scratch::new("runtime-off");
    write_mod(&scratch, "a", &manifest_text("a.mod", "1.0.0", ""));
    let mut runtime = ModRuntime::start(offer(), &Policy::Off, &[scratch.path().to_owned()], RunMode::Normal);
    assert!(runtime.loaded().is_empty());
    assert!(runtime.rejected().is_empty());
    assert_eq!(runtime.dispatch(1, 0), DispatchReport::default());
}

#[test]
fn a_library_that_will_not_open_is_refused_and_its_dependant_with_it() {
    let key = match std::env::consts::OS {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "macos",
        _ => return,
    };
    let scratch = Scratch::new("runtime-broken");
    let file = format!("broken{}", std::env::consts::DLL_SUFFIX);
    let dir = write_mod(&scratch, "a", &manifest_text("a.mod", "1.0.0", &format!("[library]\n{key} = \"{file}\"\n")));
    fs::write(dir.join(&file), b"this is not a library").expect("fake library");
    write_mod(&scratch, "b", &manifest_text("b.mod", "1.0.0", "[dependencies]\n\"a.mod\" = \"*\"\n"));
    write_mod(&scratch, "c", &manifest_text("c.mod", "1.0.0", ""));

    let runtime = ModRuntime::start(offer(), &Policy::All, &[scratch.path().to_owned()], RunMode::Normal);
    let loaded: Vec<String> = runtime.loaded().into_iter().map(|mod_| mod_.id).collect();
    assert_eq!(loaded, ["c.mod"]);
    let reasons: Vec<(&str, &RejectReason)> =
        runtime.rejected().iter().map(|rejected| (rejected.name(), &rejected.reason)).collect();
    assert!(matches!(reasons[0], ("a.mod", RejectReason::LoadFailed(_))), "{reasons:?}");
    assert_eq!(reasons[1], ("b.mod", &RejectReason::DependencyRejected { id: "a.mod".to_owned() }));
}

#[test]
fn reload_is_refused_outside_a_developer_run() {
    let scratch = Scratch::new("runtime-normal");
    write_mod(&scratch, "a", &manifest_text("a.mod", "1.0.0", ""));
    let mut runtime = ModRuntime::start(offer(), &Policy::All, &[scratch.path().to_owned()], RunMode::Normal);
    assert!(matches!(runtime.reload("a.mod"), Err(ReloadError::NotDevMode)));
}

#[test]
fn reload_names_a_mod_that_is_not_loaded_and_a_content_only_mod_with_no_library() {
    let scratch = Scratch::new("runtime-dev");
    write_mod(&scratch, "a", &manifest_text("a.mod", "1.0.0", ""));
    let mode = RunMode::Dev { cache_dir: scratch.path().join("cache") };
    let mut runtime = ModRuntime::start(offer(), &Policy::All, &[scratch.path().to_owned()], mode);
    assert!(matches!(runtime.reload("nobody.mod"), Err(ReloadError::NotLoaded(_))));
    assert!(matches!(runtime.reload("a.mod"), Err(ReloadError::ContentOnly(_))));
}

#[test]
fn a_developer_run_starts_with_an_empty_cache() {
    let scratch = Scratch::new("runtime-purge");
    let cache = scratch.path().join("cache");
    fs::create_dir_all(cache.join("old.mod")).expect("old cache");
    fs::write(cache.join("old.mod").join("0-old.dll"), b"stale").expect("stale copy");
    let mode = RunMode::Dev { cache_dir: cache.clone() };
    let _runtime = ModRuntime::start(offer(), &Policy::Off, &[scratch.path().to_owned()], mode);
    assert!(!cache.exists());
}
