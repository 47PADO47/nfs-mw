//! Parses and validates the WGSL sources with naga, so a broken shader fails `cargo test` without a GPU.

use naga::valid::{Capabilities, ValidationFlags, Validator};

const SHADERS: [(&str, &str); 5] = [
    ("scene", include_str!("shaders/scene.wgsl")),
    ("effects", include_str!("shaders/effects.wgsl")),
    ("soft_particles", include_str!("shaders/soft_particles.wgsl")),
    ("resolve", include_str!("shaders/resolve.wgsl")),
    ("fsr1", include_str!("shaders/fsr1.wgsl")),
];

fn entry_points(name: &str) -> Vec<String> {
    let source = SHADERS.iter().find(|(n, _)| *n == name).expect("known shader").1;
    let module =
        naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(source)));
    Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    module.entry_points.iter().map(|e| e.name.clone()).collect()
}

#[test]
fn every_shader_parses_and_validates() {
    for (name, _) in SHADERS {
        assert!(!entry_points(name).is_empty(), "{name} has entry points");
    }
}

#[test]
fn fsr1_exposes_the_entry_points_the_passes_use() {
    let entries = entry_points("fsr1");
    for expected in ["vs_main", "fs_easu", "fs_rcas"] {
        assert!(entries.iter().any(|e| e == expected), "{expected} in {entries:?}");
    }
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
