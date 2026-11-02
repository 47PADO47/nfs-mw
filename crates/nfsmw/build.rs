//! Link settings of the game binary.
//!
//! The window's event loop has to run on the main thread, so the main thread cannot be swapped for one with a bigger
//! stack. Windows gives it 1 MiB, and a dev build (Cranelift, large stack frames) setting up the app was at that limit:
//! one more command-line field overflowed it. The binaries get 8 MiB on MSVC targets, as the other platforms' main
//! threads have.
//!
//! The executable's icon is the game's own, so it is not in the repository: `assets/nfsmw.ico` is copied from the user's
//! install (the icon of `speed.exe`) and stays local, as the whitelist `.gitignore` keeps it out. Without the file the
//! executable keeps the default icon.
//!
//! The source revision (`NFSMW_BUILD_REVISION`, `NFSMW_BUILD_DIRTY`) is stamped for the pursuit trace's
//! `session.json`; without Git both stay unset and the trace says "unknown". As the script names the files it watches,
//! an icon copied in later is picked up on the next commit or staged change (or after touching `build.rs`).

use std::path::Path;

const ICON: &str = "assets/nfsmw.ico";

fn main() {
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    if msvc {
        println!("cargo:rustc-link-arg-bins=/STACK:8388608");
    }
    embed_icon();
    stamp_revision();
}

fn git(args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git").args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// Stamps the revision. The script reruns only when Git's `HEAD`, the branch it names or the index changes (a checkout,
/// a commit, a staged change), not on every source edit, so the dirty flag is as of the last of those.
fn stamp_revision() {
    println!("cargo:rerun-if-changed=build.rs");
    let Some(revision) = git(&["rev-parse", "HEAD"]) else { return };
    let branch = git(&["symbolic-ref", "-q", "HEAD"]);
    let watched = ["HEAD", "index"].into_iter().map(str::to_owned).chain(branch);
    for name in watched {
        // A packed branch has no file of its own; a missing path would rerun the script on every build.
        if let Some(path) = git(&["rev-parse", "--git-path", &name]).filter(|path| Path::new(path).exists()) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    println!("cargo:rustc-env=NFSMW_BUILD_REVISION={revision}");
    let dirty = git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
    println!("cargo:rustc-env=NFSMW_BUILD_DIRTY={}", u8::from(dirty));
}

fn embed_icon() {
    let windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    if !windows || !Path::new(ICON).exists() {
        return;
    }
    println!("cargo:rerun-if-changed={ICON}");
    winres::WindowsResource::new().set_icon(ICON).compile().expect("embed the executable icon");
}
