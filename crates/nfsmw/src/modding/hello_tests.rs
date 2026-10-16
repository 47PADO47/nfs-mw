//! The host end to end: build the `hello` example with cargo, start it through the game's [`Mods`](super::Mods), and
//! check that it loads and receives the frame events. The build makes the test slow, so it is ignored; run it with
//! `cargo test -p nfsmw modding -- --ignored`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use blackbox_modding::host::{Policy, RunMode};

use super::backends::{self, support};
use super::{Mods, policy};

/// Removes the folder when dropped.
struct Scratch {
    path: PathBuf,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The platform's file name of the hello library, as cargo produces it.
fn library_file_name() -> String {
    format!("{}hello_mod{}", env::consts::DLL_PREFIX, env::consts::DLL_SUFFIX)
}

/// Build `hello-mod` with the profile of this test binary, and return the library it produced.
fn build_hello() -> PathBuf {
    let mut command = Command::new(env!("CARGO"));
    command.current_dir(workspace_root()).args(["build", "-p", "hello-mod"]);
    if !cfg!(debug_assertions) {
        command.arg("--release");
    }
    let status = command.status().expect("run cargo build -p hello-mod");
    assert!(status.success(), "cargo build -p hello-mod failed");

    let exe = env::current_exe().expect("test binary path");
    let profile_dir = exe.parent().and_then(Path::parent).expect("target profile folder");
    let library = profile_dir.join(library_file_name());
    assert!(library.is_file(), "built library not found at {}", library.display());
    library
}

#[test]
#[ignore = "builds examples/mods/hello with cargo, which is slow; run with --ignored"]
fn the_hello_mod_loads_and_receives_the_frame_events() {
    let library = build_hello();
    let root = env::temp_dir().join(format!("nfsmw-hello-{}", std::process::id()));
    let scratch = Scratch { path: root.clone() };
    let folder = root.join("example.hello");
    fs::create_dir_all(&folder).expect("the mod folder");
    fs::copy(workspace_root().join("examples").join("mods").join("hello").join("mod.toml"), folder.join("mod.toml"))
        .expect("copy the manifest");
    fs::copy(&library, folder.join(library_file_name())).expect("copy the library");

    // The game offers its four interfaces. Hello reads the clock on its first event, and the frame copies the scene's
    // clock in before the mods run, so this test copies a clock in before the first frame: 13:30 at 150 real seconds
    // per hour.
    support::captured_log();
    let interfaces = backends::interfaces();
    let bridge = interfaces.bridge.clone();
    let mut mods = Mods::start(policy::offer(), interfaces, &Policy::All, std::slice::from_ref(&root), RunMode::Normal);

    let loaded = mods.loaded();
    assert_eq!(loaded.len(), 1, "rejected: {:?}", mods.rejected());
    assert_eq!(loaded[0].id, "example.hello");
    bridge.sync(&mut support::Recorder::at(13.5, 150.0));
    let frame = mods.frame_start(1.0 / 60.0);
    assert_eq!((frame.delivered, frame.faults), (1, 0), "FRAME_START reaches the mod");
    let read = support::captured_log().into_iter().find(|line| line.starts_with("[example.hello] hello: time"));
    assert_eq!(
        read.as_deref(),
        Some("[example.hello] hello: time 13.50 h, 150 s per hour, frozen false"),
        "hello reads the clock through bb.time on its first event"
    );
    let tick = mods.scene_tick(1.0 / 60.0, 1.0, false);
    assert_eq!((tick.delivered, tick.faults), (1, 0), "SCENE_TICK reaches the mod");

    // Hello logs the player's speed on frame 100 through bb.vehicles. A car on the road is copied in, and the frames run on
    // to 100 (the first one is above).
    bridge.sync(&mut support::Recorder::driving([0.0, 0.0, 0.0], 0.0));
    for _ in 2..=100 {
        mods.frame_start(1.0 / 60.0);
    }
    let speed =
        support::captured_log().into_iter().find(|line| line.starts_with("[example.hello] hello: player speed"));
    assert_eq!(
        speed.as_deref(),
        Some("[example.hello] hello: player speed 20.00 m/s at frame 100"),
        "hello reads the player's speed through bb.vehicles on frame 100"
    );

    // Dropping the mods sends MOD_UNLOADING, then destroys the mod.
    drop(mods);
    drop(scratch);
}
