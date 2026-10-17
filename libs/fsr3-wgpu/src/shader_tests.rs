//! Parses and validates the WGSL sources with naga, so a broken shader fails `cargo test` without a GPU.

use naga::{
    ImageClass, StorageFormat, TypeInner,
    valid::{Capabilities, ValidationFlags, Validator},
};
use wgpu::TextureFormat;

use crate::{
    config::Fsr3Config,
    constants::{CONSTANTS_SIZE, Constants},
    formats::Intermediates,
    shaders::{COMMON, Pass, module_source},
};

/// Every combination of the 16-bit and 32-bit intermediate formats, as a device may give any of them.
fn variants() -> Vec<Intermediates> {
    let mut all = Vec::new();
    for pair in [TextureFormat::Rg32Float, TextureFormat::Rg16Float] {
        for single in [TextureFormat::R32Float, TextureFormat::R16Float] {
            for lock in [TextureFormat::R32Float, TextureFormat::R16Float] {
                all.push(Intermediates { pair, single, lock });
            }
        }
    }
    all
}

fn parse(pass: Pass, format: TextureFormat, formats: Intermediates) -> naga::Module {
    let source = module_source(pass, format, formats);
    let module = naga::front::wgsl::parse_str(&source)
        .unwrap_or_else(|e| panic!("{}: {}", pass.file_name(), e.emit_to_string(&source)));
    Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}: {}", pass.file_name(), e.emit_to_string(&source)));
    module
}

#[test]
fn every_pass_parses_and_validates_for_every_output_format_and_variant() {
    for formats in variants() {
        for format in Fsr3Config::OUTPUT_FORMATS {
            for pass in Pass::ALL {
                parse(pass, format, formats);
            }
        }
    }
}

#[test]
fn every_pass_exposes_the_entry_points_the_context_uses() {
    for pass in Pass::ALL {
        let module = parse(pass, TextureFormat::Rgba16Float, Intermediates::FULL);
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
    let module = parse(Pass::Accumulate, TextureFormat::Rgba16Float, Intermediates::FULL);
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
    let module = parse(Pass::PrepareInputs, TextureFormat::Rgba16Float, Intermediates::FULL);
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

fn naga_format(format: TextureFormat) -> StorageFormat {
    match format {
        TextureFormat::R16Float => StorageFormat::R16Float,
        TextureFormat::Rg16Float => StorageFormat::Rg16Float,
        TextureFormat::R32Float => StorageFormat::R32Float,
        TextureFormat::Rg32Float => StorageFormat::Rg32Float,
        TextureFormat::Rgba16Float => StorageFormat::Rgba16Float,
        other => panic!("{other:?} is not checked here"),
    }
}

/// The storage format a module declares for the global storage texture `name`.
fn storage_format(module: &naga::Module, name: &str) -> StorageFormat {
    let (_, global) = module
        .global_variables
        .iter()
        .find(|(_, g)| g.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("{name} is declared"));
    let TypeInner::Image { class: ImageClass::Storage { format, .. }, .. } = &module.types[global.ty].inner else {
        panic!("{name} is a storage texture")
    };
    *format
}

/// Each intermediate image is declared in the format the context chose, and the depth images stay 32-bit.
#[test]
fn every_storage_texture_has_the_format_of_its_variant() {
    for formats in variants() {
        let prepare = parse(Pass::PrepareInputs, TextureFormat::Rgba16Float, formats);
        assert_eq!(storage_format(&prepare, "out_dilated_motion_vectors"), naga_format(formats.pair));
        assert_eq!(storage_format(&prepare, "out_current_luma"), naga_format(formats.single));
        assert_eq!(storage_format(&prepare, "out_dilated_depth"), StorageFormat::R32Float);
        assert_eq!(storage_format(&prepare, "out_farthest_depth"), StorageFormat::R32Float);

        let pyramid = parse(Pass::ShadingChangePyramid, TextureFormat::Rgba16Float, formats);
        assert_eq!(storage_format(&pyramid, "out_level"), naga_format(formats.pair));

        let shading = parse(Pass::ShadingChange, TextureFormat::Rgba16Float, formats);
        assert_eq!(storage_format(&shading, "out_shading_change"), naga_format(formats.single));

        let instability = parse(Pass::LumaInstability, TextureFormat::Rgba16Float, formats);
        assert_eq!(storage_format(&instability, "out_luma_instability"), naga_format(formats.single));

        let reactivity = parse(Pass::PrepareReactivity, TextureFormat::Rgba16Float, formats);
        assert_eq!(storage_format(&reactivity, "out_accumulation"), naga_format(formats.single));
        assert_eq!(storage_format(&reactivity, "new_locks"), naga_format(formats.lock));

        let accumulate = parse(Pass::Accumulate, TextureFormat::Rgba8Unorm, formats);
        assert_eq!(storage_format(&accumulate, "new_locks"), naga_format(formats.lock));
        assert_eq!(storage_format(&accumulate, "out_upscaled"), StorageFormat::Rgba8Unorm);

        let luma = parse(Pass::LumaPyramid, TextureFormat::Rgba16Float, formats);
        assert_eq!(storage_format(&luma, "out_farthest_depth_mip1"), StorageFormat::R32Float);
    }
}
