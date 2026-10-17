//! The pure parts of the mod host: policy, roots, layers, event payloads, the console listing and the flags.

use std::fs;
use std::mem::size_of;
use std::path::PathBuf;

use blackbox_modding::abi::{BbFrameStart, BbSceneTick};
use blackbox_modding::host::{Candidate, ModSummary, Policy, RejectReason, Rejected, RunMode, parse};
use clap::Parser;

use super::{console, events, layers, policy};
use crate::cli::Cli;
use crate::settings::mods::{ModsMode, ModsSettings};

/// A folder under the temp directory, removed when dropped.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let nanos =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let path = std::env::temp_dir().join(format!("nfsmw-modding-{tag}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).expect("create scratch folder");
        Scratch { path }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn candidate(id: &str, priority: i32, dir: PathBuf) -> Candidate {
    let text = format!(
        "[mod]\nid = \"{id}\"\nname = \"{id}\"\nversion = \"1.0.0\"\n[api]\nblackbox_modding = \"0.1\"\n\
         [games]\nsupported = [\"*\"]\n[content]\npriority = {priority}\n"
    );
    Candidate { manifest: parse(&text).expect("the test manifest is valid"), dir, library: None }
}

fn summary(id: &str, quarantined: bool, faults: u32) -> ModSummary {
    ModSummary { id: id.to_owned(), version: "1.0.0".to_owned(), library: None, sha256: None, faults, quarantined }
}

/// The command-line layer of a `play` invocation, parsed the way the game parses it.
fn play_layer(args: &[&str]) -> crate::cli::ViewArgs {
    let mut argv = vec!["nfsmw", "play"];
    argv.extend_from_slice(args);
    match Cli::try_parse_from(argv).expect("the flags parse").command {
        Some(crate::cli::Command::Play { view, .. }) => view,
        _ => panic!("expected the play command"),
    }
}

#[test]
fn the_policy_follows_the_mode() {
    let all = ModsSettings { mode: ModsMode::All, allowlist: vec!["a".into()] };
    assert_eq!(policy::policy(&all), Policy::All);
    let off = ModsSettings { mode: ModsMode::Off, ..all.clone() };
    assert_eq!(policy::policy(&off), Policy::Off);
    let listed = ModsSettings { mode: ModsMode::Allowlist, allowlist: vec!["b".into(), "a".into()] };
    let Policy::Allowlist(ids) = policy::policy(&listed) else {
        panic!("an allowlist mode gives an allowlist policy");
    };
    assert_eq!(ids.into_iter().collect::<Vec<_>>(), ["a", "b"]);
}

#[test]
fn no_mods_turns_the_mods_off_on_the_command_line_layer() {
    let view = play_layer(&["--no-mods"]);
    assert_eq!(view.settings_layer().mods, Some(ModsMode::Off));
    assert_eq!(play_layer(&[]).settings_layer().mods, None);
}

#[test]
fn the_developer_flags_are_read() {
    let view = play_layer(&["--mod-dev", "--mods-dir", "extra"]);
    assert!(view.mod_dev);
    assert_eq!(view.mods_dir, Some(PathBuf::from("extra")));
}

#[test]
fn the_roots_are_the_mods_folder_then_the_extra_folder() {
    let config = PathBuf::from("cfg").join("config.toml");
    let extra = PathBuf::from("extra");
    assert_eq!(policy::roots(Some(&config), None), [PathBuf::from("cfg").join("mods")]);
    assert_eq!(policy::roots(Some(&config), Some(&extra)), [PathBuf::from("cfg").join("mods"), extra.clone()]);
    assert_eq!(policy::roots(None, Some(&extra)), [extra]);
}

#[test]
fn a_normal_run_does_not_copy_the_libraries() {
    assert_eq!(policy::run_mode(false), RunMode::Normal);
    match policy::run_mode(true) {
        RunMode::Dev { cache_dir } => assert!(cache_dir.ends_with("mod-cache")),
        RunMode::Normal => panic!("a developer run copies the libraries"),
    }
}

#[test]
fn layers_follow_priority_then_load_order_and_skip_mods_without_files() {
    let scratch = Scratch::new("layers");
    let a = scratch.path.join("a");
    let b = scratch.path.join("b");
    let c = scratch.path.join("c");
    fs::create_dir_all(a.join("files")).expect("a/files");
    fs::create_dir_all(b.join("files")).expect("b/files");
    fs::create_dir_all(&c).expect("c");
    let candidates = [candidate("a.mod", 5, a), candidate("b.mod", 0, b), candidate("c.mod", 0, c)];

    let loaded: Vec<String> = ["a.mod", "b.mod", "c.mod"].map(String::from).into();
    let ids: Vec<String> = layers::plan(&loaded, &candidates).into_iter().map(|layer| layer.id).collect();
    assert_eq!(ids, ["b.mod", "a.mod"]);
}

#[test]
fn equal_priorities_keep_the_load_order() {
    let scratch = Scratch::new("ties");
    let x = scratch.path.join("x");
    let y = scratch.path.join("y");
    fs::create_dir_all(x.join("files")).expect("x/files");
    fs::create_dir_all(y.join("files")).expect("y/files");
    let candidates = [candidate("x.mod", 0, x), candidate("y.mod", 0, y)];

    let loaded: Vec<String> = ["y.mod", "x.mod"].map(String::from).into();
    let ids: Vec<String> = layers::plan(&loaded, &candidates).into_iter().map(|layer| layer.id).collect();
    assert_eq!(ids, ["y.mod", "x.mod"]);
}

#[test]
fn the_frame_start_payload_carries_its_size_and_the_real_time() {
    let payload = events::frame_start_payload(0.25);
    assert_eq!(payload.size as usize, size_of::<BbFrameStart>());
    assert_eq!(payload.dt_real, 0.25);
}

#[test]
fn the_scene_tick_scales_the_real_time_and_stops_while_paused() {
    let running = events::scene_tick_payload(0.5, 2.0, false);
    assert_eq!(running.size as usize, size_of::<BbSceneTick>());
    assert_eq!((running.dt_sim, running.time_scale, running.paused), (1.0, 2.0, 0));
    let paused = events::scene_tick_payload(0.5, 2.0, true);
    assert_eq!((paused.dt_sim, paused.paused), (0.0, 1));
}

#[test]
fn the_unloading_payload_borrows_the_id_for_its_length() {
    let payload = events::unloading_payload("example.hello");
    assert_eq!(payload.mod_id.len, "example.hello".len() as u64);
}

#[test]
fn the_listing_names_each_mod_and_its_state() {
    let loaded = [summary("example.hello", false, 0), summary("bad.mod", true, 5)];
    let rejected = [Rejected {
        id: Some("app.mod".to_owned()),
        folder: "app.mod".to_owned(),
        dir: PathBuf::from("app.mod"),
        reason: RejectReason::DependencyMissing { id: "nobody.base".to_owned() },
    }];
    let text = console::lines(&loaded, &rejected);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "example.hello 1.0.0 loaded");
    assert_eq!(lines[1], "bad.mod 1.0.0 quarantined after 5 faults");
    assert_eq!(lines[2], rejected[0].summary());
}

#[test]
fn the_listing_says_when_there_are_no_mods() {
    assert_eq!(console::lines(&[], &[]), "no mods found in the mods folders");
    assert!(console::listing(None).contains("without them"));
}
