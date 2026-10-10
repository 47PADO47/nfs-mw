//! Parses and validates the compiled WGSL (the build script's expansion of the WESL sources, plus the
//! shaders with no `SRGB_TARGET` conditional) with naga, so a broken shader fails `cargo test` without a
//! GPU.

use naga::valid::{Capabilities, ValidationFlags, Validator};

const SHADERS: [(&str, &str); 7] = [
    ("ui", include_str!(concat!(env!("OUT_DIR"), "/ui.wgsl"))),
    ("ui (srgb)", include_str!(concat!(env!("OUT_DIR"), "/ui_srgb.wgsl"))),
    ("effects", crate::EFFECTS_WGSL),
    ("effects (srgb)", crate::EFFECTS_WGSL_SRGB),
    ("soft_particles", include_str!(concat!(env!("OUT_DIR"), "/soft_particles.wgsl"))),
    ("soft_particles (srgb)", include_str!(concat!(env!("OUT_DIR"), "/soft_particles_srgb.wgsl"))),
    // `fsr1.wgsl` has no `SRGB_TARGET` conditional and stays plain WGSL.
    ("fsr1", include_str!("shaders/fsr1.wgsl")),
];

fn entry_points(source: &str, name: &str) -> Vec<String> {
    let module =
        naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(source)));
    Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    module.entry_points.iter().map(|e| e.name.clone()).collect()
}

fn assert_entries(name: &str, expected: &[&str]) {
    let source = SHADERS.iter().find(|(n, _)| *n == name).expect("known shader").1;
    let entries = entry_points(source, name);
    for entry in expected {
        assert!(entries.iter().any(|e| e == entry), "{name}: {entry} in {entries:?}");
    }
}

#[test]
fn every_shader_parses_and_validates() {
    for (name, source) in SHADERS {
        assert!(!entry_points(source, name).is_empty(), "{name} has entry points");
    }
}

#[test]
fn the_passes_find_their_entry_points() {
    assert_entries("ui", &["vs_main", "fs_main"]);
    assert_entries("ui (srgb)", &["vs_main", "fs_main"]);
    assert_entries(
        "effects",
        &["vs_main", "fs_surface", "fs_particle", "fs_streak", "fs_glow", "fs_textured", "fs_textured_alpha"],
    );
    assert_entries(
        "effects (srgb)",
        &["vs_main", "fs_surface", "fs_particle", "fs_streak", "fs_glow", "fs_textured", "fs_textured_alpha"],
    );
    assert_entries("soft_particles", &["vs_main", "fs_main"]);
    assert_entries("soft_particles (srgb)", &["vs_main", "fs_main"]);
    assert_entries("fsr1", &["vs_main", "fs_easu", "fs_rcas"]);
}

#[test]
fn fsr1_keeps_the_licence_notice() {
    let source = SHADERS.iter().find(|(n, _)| *n == "fsr1").unwrap().1;
    for needle in [
        "Copyright (c) 2021 Advanced Micro Devices, Inc.",
        "Copyright (c) 2014 Michal Drobot",
        "Permission is hereby granted, free of charge",
        "THE SOFTWARE IS PROVIDED \"AS IS\"",
    ] {
        assert!(source.contains(needle), "the notice stays in the file: {needle}");
    }
}
