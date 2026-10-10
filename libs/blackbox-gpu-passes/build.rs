//! Expands the WESL shaders (the `SRGB_TARGET` conditional) to plain WGSL at build time, one file
//! per value. Each `.wesl` source is self-contained (no cross-file imports), so a single in-memory
//! module is enough; no filesystem resolution of the sources being compiled is needed. `wesl` is a
//! build dependency only: the crate stays as light at runtime as it was with plain `.wgsl` files.

use std::borrow::Cow;
use std::env;
use std::fs;
use std::path::PathBuf;

use wesl::pass::Features;
use wesl::resolver::VirtualResolver;
use wesl::syntax::ModulePath;
use wesl::{CompileOptions, ManglerKind};

/// Shaders with a `SRGB_TARGET` conditional, named after their file in `src/shaders/`.
const SHADERS: [&str; 3] = ["effects", "ui", "soft_particles"];

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
    for name in SHADERS {
        let source_path = format!("src/shaders/{name}.wesl");
        println!("cargo:rerun-if-changed={source_path}");
        let source = fs::read_to_string(&source_path).unwrap_or_else(|e| panic!("{source_path}: {e}"));
        write_variant(&out_dir, name, &source, false);
        write_variant(&out_dir, name, &source, true);
    }
}

/// Compile `source` with `SRGB_TARGET` set to `srgb` and write the resulting WGSL to `out_dir`.
fn write_variant(out_dir: &std::path::Path, name: &str, source: &str, srgb: bool) {
    let wgsl = expand(source, srgb);
    let file_name = match srgb {
        true => format!("{name}_srgb.wgsl"),
        false => format!("{name}.wgsl"),
    };
    fs::write(out_dir.join(&file_name), wgsl).unwrap_or_else(|e| panic!("{file_name}: {e}"));
}

/// Compile one self-contained `.wesl` source with `SRGB_TARGET` resolved to `srgb`, no mangling (there
/// is only one module, so there are no name collisions to avoid) and no other WESL passes needed.
fn expand(source: &str, srgb: bool) -> String {
    let path = ModulePath::from_path("/main");
    let mut resolver = VirtualResolver::new();
    resolver.add_module(path.clone(), Cow::Borrowed(source));
    let mut features = Features::new();
    features.set("SRGB_TARGET", srgb);
    let options = CompileOptions { condcomp: true, features, mangler: ManglerKind::None, ..Default::default() };
    let result = wesl::compile(&path, &options, &resolver).unwrap_or_else(|e| panic!("{path}: {e}"));
    result.to_string()
}
