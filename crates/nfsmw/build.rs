//! Link settings of the game binary.
//!
//! The window's event loop has to run on the main thread, so the main thread cannot be swapped for one with a bigger
//! stack. Windows gives it 1 MiB, and a dev build (Cranelift, large stack frames) setting up the app was at that limit:
//! one more command-line field overflowed it. The binaries get 8 MiB on MSVC targets, as the other platforms' main
//! threads have.

fn main() {
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    if msvc {
        println!("cargo:rustc-link-arg-bins=/STACK:8388608");
    }
}
