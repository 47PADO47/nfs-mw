//! Parses and validates the WGSL of the three passes with naga, so a broken shader fails `cargo test` without a GPU.

use naga::valid::{Capabilities, ValidationFlags, Validator};

use crate::shaders::{BLEND_WEIGHT_WGSL, EDGE_DETECT_WGSL, NEIGHBORHOOD_WGSL};

/// Each pass with the bind-group bindings its pipeline layout declares (`bindings.rs`), in group 0.
const PASSES: [(&str, &str, &[u32]); 3] = [
    ("edge detect", EDGE_DETECT_WGSL, &[0, 1, 2]),
    ("blend weight", BLEND_WEIGHT_WGSL, &[0, 1, 2, 3, 4]),
    ("neighborhood blending", NEIGHBORHOOD_WGSL, &[0, 1, 2, 3]),
];

fn parse_and_validate(name: &str, source: &str) -> naga::Module {
    let module =
        naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(source)));
    Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    module
}

#[test]
fn every_pass_parses_and_validates() {
    for (name, source, _) in PASSES {
        parse_and_validate(name, source);
    }
}

#[test]
fn every_pass_has_a_vertex_and_a_fragment_entry_point() {
    for (name, source, _) in PASSES {
        let module = parse_and_validate(name, source);
        let stages: Vec<_> = module.entry_points.iter().map(|e| (e.name.as_str(), e.stage)).collect();
        assert!(stages.contains(&("vs_main", naga::ShaderStage::Vertex)), "{name}: {stages:?}");
        assert!(stages.contains(&("fs_main", naga::ShaderStage::Fragment)), "{name}: {stages:?}");
    }
}

#[test]
fn every_pass_declares_the_bindings_its_layout_expects() {
    for (name, source, expected) in PASSES {
        let module = parse_and_validate(name, source);
        let mut bindings: Vec<u32> = module
            .global_variables
            .iter()
            .filter_map(|(_, var)| var.binding.as_ref())
            .map(|b| {
                assert_eq!(b.group, 0, "{name}: every binding is in group 0");
                b.binding
            })
            .collect();
        bindings.sort_unstable();
        assert_eq!(bindings, expected, "{name}");
    }
}
