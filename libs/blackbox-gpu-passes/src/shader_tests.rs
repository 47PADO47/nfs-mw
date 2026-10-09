//! Parses and validates the WGSL sources with naga, so a broken shader fails `cargo test` without a GPU.

use naga::valid::{Capabilities, ValidationFlags, Validator};

const SHADERS: [(&str, &str); 1] = [("ui", include_str!("shaders/ui.wgsl"))];

fn entry_points(name: &str) -> Vec<String> {
    let source = SHADERS.iter().find(|(n, _)| *n == name).expect("known shader").1;
    let module =
        naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(source)));
    Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    module.entry_points.iter().map(|e| e.name.clone()).collect()
}

fn assert_entries(name: &str, expected: &[&str]) {
    let entries = entry_points(name);
    for entry in expected {
        assert!(entries.iter().any(|e| e == entry), "{name}: {entry} in {entries:?}");
    }
}

#[test]
fn every_shader_parses_and_validates() {
    for (name, _) in SHADERS {
        assert!(!entry_points(name).is_empty(), "{name} has entry points");
    }
}

#[test]
fn the_passes_find_their_entry_points() {
    assert_entries("ui", &["vs_main", "fs_main"]);
}
