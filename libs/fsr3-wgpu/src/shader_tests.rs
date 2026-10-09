//! Parses and validates the WGSL sources with naga, so a broken shader fails `cargo test` without a GPU.

use naga::{
    TypeInner,
    valid::{Capabilities, ValidationFlags, Validator},
};
use wgpu::TextureFormat;

use crate::{
    config::Fsr3Config,
    constants::{CONSTANTS_SIZE, Constants},
    shaders::{COMMON, Pass, module_source},
};

fn parse(pass: Pass, format: TextureFormat) -> naga::Module {
    let source = module_source(pass, format);
    let module = naga::front::wgsl::parse_str(&source)
        .unwrap_or_else(|e| panic!("{}: {}", pass.file_name(), e.emit_to_string(&source)));
    Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}: {}", pass.file_name(), e.emit_to_string(&source)));
    module
}

#[test]
fn every_pass_parses_and_validates_for_every_output_format() {
    for format in Fsr3Config::OUTPUT_FORMATS {
        for pass in Pass::ALL {
            parse(pass, format);
        }
    }
}

#[test]
fn every_pass_exposes_the_entry_points_the_context_uses() {
    for pass in Pass::ALL {
        let module = parse(pass, TextureFormat::Rgba16Float);
        let names: Vec<_> = module.entry_points.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names.len(), pass.entry_points().len(), "{pass:?}: {names:?}");
        for expected in pass.entry_points() {
            assert!(names.contains(expected), "{pass:?} lacks {expected}: {names:?}");
        }
        for entry in &module.entry_points {
            assert_eq!(entry.stage, naga::ShaderStage::Compute, "{pass:?} {}", entry.name);
        }
    }
}

#[test]
fn the_pipeline_overridable_options_are_declared() {
    let module = parse(Pass::Accumulate, TextureFormat::Rgba16Float);
    let names: Vec<_> = module.overrides.iter().filter_map(|(_, o)| o.name.clone()).collect();
    for option in crate::shaders::OVERRIDE_NAMES {
        assert!(names.iter().any(|n| n == option), "{option} in {names:?}");
    }
}

#[test]
fn every_file_keeps_the_amd_licence_notice() {
    let sources = std::iter::once(("common.wgsl", COMMON)).chain(Pass::ALL.map(|p| (p.file_name(), p.source())));
    for (name, source) in sources {
        for needle in [
            "Copyright (C) 2024 Advanced Micro Devices, Inc.",
            "Permission is hereby granted, free of charge",
            "THE SOFTWARE IS PROVIDED \"AS IS\"",
            "FidelityFX-SDK",
        ] {
            assert!(source.contains(needle), "{name} keeps the notice: {needle}");
        }
    }
}

#[test]
fn files_stay_under_the_size_limit_with_room_to_spare() {
    let sources = std::iter::once(("common.wgsl", COMMON)).chain(Pass::ALL.map(|p| (p.file_name(), p.source())));
    for (name, source) in sources {
        assert!(source.lines().count() <= 500, "{name} has {} lines", source.lines().count());
    }
}

/// The offsets and size of the uniform block in the shader must be the ones of the Rust struct.
#[test]
fn the_uniform_block_has_the_layout_of_the_rust_struct() {
    let module = parse(Pass::PrepareInputs, TextureFormat::Rgba16Float);
    let (_, ty) =
        module.types.iter().find(|(_, t)| t.name.as_deref() == Some("Constants")).expect("the Constants struct");
    let TypeInner::Struct { members, span } = &ty.inner else { panic!("Constants is a struct") };
    assert_eq!(*span as usize, CONSTANTS_SIZE);

    use std::mem::offset_of;
    let expected = [
        ("render_size", offset_of!(Constants, render_size)),
        ("previous_render_size", offset_of!(Constants, previous_render_size)),
        ("upscale_size", offset_of!(Constants, upscale_size)),
        ("previous_upscale_size", offset_of!(Constants, previous_upscale_size)),
        ("max_render_size", offset_of!(Constants, max_render_size)),
        ("max_upscale_size", offset_of!(Constants, max_upscale_size)),
        ("device_to_view_depth", offset_of!(Constants, device_to_view_depth)),
        ("jitter", offset_of!(Constants, jitter)),
        ("previous_jitter", offset_of!(Constants, previous_jitter)),
        ("motion_vector_scale", offset_of!(Constants, motion_vector_scale)),
        ("downscale_factor", offset_of!(Constants, downscale_factor)),
        ("motion_vector_jitter_cancellation", offset_of!(Constants, motion_vector_jitter_cancellation)),
        ("tan_half_fov", offset_of!(Constants, tan_half_fov)),
        ("jitter_phase_count", offset_of!(Constants, jitter_phase_count)),
        ("delta_time", offset_of!(Constants, delta_time)),
        ("delta_pre_exposure", offset_of!(Constants, delta_pre_exposure)),
        ("view_space_to_meters", offset_of!(Constants, view_space_to_meters)),
        ("frame_index", offset_of!(Constants, frame_index)),
        ("velocity_factor", offset_of!(Constants, velocity_factor)),
        ("reactiveness_scale", offset_of!(Constants, reactiveness_scale)),
        ("shading_change_scale", offset_of!(Constants, shading_change_scale)),
        ("accumulation_added_per_frame", offset_of!(Constants, accumulation_added_per_frame)),
        ("min_disocclusion_accumulation", offset_of!(Constants, min_disocclusion_accumulation)),
        ("rcas_lobe", offset_of!(Constants, rcas_lobe)),
    ];
    assert_eq!(members.len(), expected.len() + 2, "every field but the padding is checked");
    for (member, (name, offset)) in members.iter().zip(expected) {
        assert_eq!(member.name.as_deref(), Some(name));
        assert_eq!(member.offset as usize, offset, "{name}");
    }
}
