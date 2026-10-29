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

use std::path::Path;

const ICON: &str = "assets/nfsmw.ico";

fn main() {
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    if msvc {
        println!("cargo:rustc-link-arg-bins=/STACK:8388608");
    }
    embed_icon();
}

fn embed_icon() {
    let windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    if !windows || !Path::new(ICON).exists() {
        return;
    }
    println!("cargo:rerun-if-changed={ICON}");
    winres::WindowsResource::new().set_icon(ICON).compile().expect("embed the executable icon");
}
